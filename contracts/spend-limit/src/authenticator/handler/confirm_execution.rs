use cosmwasm_std::{DepsMut, Env, Response};
use osmosis_authenticators::ConfirmExecutionRequest;

use crate::authenticator::common::try_spend_all;
use crate::spend_limit::{calculate_spent_coins, SpendLimitParams};

use crate::state::{PRE_EXEC_BALANCES, PRICE_RESOLUTION_CONFIG, SPENDINGS, UNTRACKED_SPENT_FEES};
use crate::ContractError;

use super::validate_and_parse_params;

pub fn confirm_execution(
    mut deps: DepsMut,
    env: Env,
    ConfirmExecutionRequest {
        authenticator_id,
        account,
        authenticator_params,
        ..
    }: ConfirmExecutionRequest,
) -> Result<Response, ContractError> {
    let params: SpendLimitParams = validate_and_parse_params(authenticator_params)?;

    let spend_limit_key = (&account, authenticator_id.as_str());

    // get the pre_exec balance for this key
    let pre_exec_balances = PRE_EXEC_BALANCES.load(deps.storage, spend_limit_key)?;

    // query all the balances of the account
    let post_exec_balances = deps.querier.query_all_balances(&account)?;

    let pre_exec_balances = pre_exec_balances.try_into()?;
    let post_exec_balances = post_exec_balances.try_into()?;
    let spent_coins = calculate_spent_coins(pre_exec_balances, post_exec_balances)?;

    let mut spending = SPENDINGS.load(deps.storage, spend_limit_key)?;

    let conf = PRICE_RESOLUTION_CONFIG.load(deps.storage)?;

    try_spend_all(
        deps.branch(),
        &mut spending,
        spent_coins,
        &conf,
        params.limit,
        &params.reset_period,
        env.block.time,
    )?;

    // save the updated spending
    SPENDINGS.save(deps.storage, spend_limit_key, &spending)?;

    // clean up the pre_exec balance
    PRE_EXEC_BALANCES.remove(deps.storage, spend_limit_key);

    // Clean up untracked spent fee, since the transaction is successful
    // fee has already been captured as part of the balance difference
    UNTRACKED_SPENT_FEES.remove(deps.storage, spend_limit_key);

    Ok(Response::new()
        .add_attribute("action", "confirm_execution")
        .add_attribute("spent", spending.value_spent_in_period)
        .add_attribute("limit", params.limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::period::Period;
    use crate::{
        price::PriceResolutionConfig,
        spend_limit::{SpendLimitError, SpendLimitParams, Spending},
        state::UNTRACKED_SPENT_FEES,
    };
    use cosmwasm_std::{
        testing::{mock_dependencies_with_balances, mock_env},
        to_json_binary, Addr, Binary, Coin, Response, Uint128,
    };
    use osmosis_authenticators::ConfirmExecutionRequest;
    use rstest::rstest;

    #[rstest]
    #[case::spend_at_limit(1000, 500, 500, vec![Coin::new(1000_000_000, "uosmo")], Ok(Response::new()
        .add_attribute("action", "confirm_execution")
        .add_attribute("spent", spent.to_string())
        .add_attribute("limit", limit.to_string())
    ))]
    #[case::spend_over_limit(1000, 500, 501, vec![Coin::new(1000_000_000, "uosmo")], Err(SpendLimitError::overspend(500, 501).into()))]
    fn test_confirm_execution_only_spends_quoted_denom(
        #[case] initial_balance: u128,
        #[case] limit: u128,
        #[case] spent: u128,
        #[case] untracked_spent_fee: Vec<Coin>,
        #[case] expected: Result<Response, ContractError>,
    ) {
        let fixed_balance = Coin::new(500, "uosmo");
        // Setup the environment
        let mut deps = mock_dependencies_with_balances(&[(
            "account",
            &[
                Coin::new(initial_balance - spent, "uusdc"),
                fixed_balance.clone(),
            ],
        )]);

        let key = (&Addr::unchecked("account"), "2");

        UNTRACKED_SPENT_FEES
            .save(&mut deps.storage, key, &untracked_spent_fee)
            .unwrap();

        PRE_EXEC_BALANCES
            .save(
                deps.as_mut().storage,
                key,
                &vec![Coin::new(initial_balance, "uusdc"), fixed_balance],
            )
            .unwrap();

        SPENDINGS
            .save(&mut deps.storage, key, &Spending::default())
            .unwrap();

        PRICE_RESOLUTION_CONFIG
            .save(
                deps.as_mut().storage,
                &PriceResolutionConfig {
                    quote_denom: "uusdc".to_string(),
                    staleness_threshold: 3_600_000_000_000u64.into(), // 1h
                    twap_duration: 3_600_000_000_000u64.into(),       // 1h
                },
            )
            .unwrap();

        // Confirm the execution
        let confirm_execution_request = ConfirmExecutionRequest {
            authenticator_id: "2".to_string(),
            account: Addr::unchecked("account"),
            fee_payer: Addr::unchecked("account"),
            fee_granter: None,
            fee: vec![],
            authenticator_params: Some(
                to_json_binary(&SpendLimitParams {
                    limit: Uint128::new(limit),
                    reset_period: Period::Day,
                    time_limit: None,
                })
                .unwrap(),
            ),
            msg: osmosis_authenticators::Any {
                type_url: "".to_string(),
                value: Binary::default(),
            },
            msg_index: 0,
        };

        let res = confirm_execution(deps.as_mut(), mock_env(), confirm_execution_request);
        match expected {
            Ok(expected_res) => {
                assert_eq!(res.unwrap(), expected_res);

                // Verify that the spending is updated correctly
                let spending = SPENDINGS.load(deps.as_ref().storage, key).unwrap();
                assert_eq!(
                    spending,
                    Spending {
                        value_spent_in_period: spent.into(),
                        last_spent_at: mock_env().block.time
                    }
                );

                // verify that the untracked spent fee is cleaned up
                let untracked_spent_fee = UNTRACKED_SPENT_FEES
                    .may_load(deps.as_ref().storage, key)
                    .unwrap();
                assert_eq!(untracked_spent_fee, None);
            }
            Err(expected_err) => {
                assert_eq!(res.unwrap_err(), expected_err);

                // verify that untracked spent fee is not cleaned up
                assert_eq!(
                    UNTRACKED_SPENT_FEES
                        .may_load(deps.as_ref().storage, key)
                        .unwrap(),
                    Some(untracked_spent_fee)
                );
            }
        }
    }
}
