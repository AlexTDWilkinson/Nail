//! Finance module stdlib registry entries.
//!
//! The time value of money over plain floats, for planning and comparison,
//! with the money module handling cent-exact bookkeeping. Every rate is the
//! yearly percentage as people quote it, so 6.0 means six percent, the same
//! convention as money_loan_payment.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    simple_fns! { m, Finance:
        "finance_present_value" => "std_lib::finance::present_value", (future_value: f, rate_percent: f, periods: i) -> (f!e),
            "Returns what a future amount is worth today, discounted at the yearly rate over the given periods. The rate is the yearly percentage as people quote it, 6.0 means six percent, matching money_loan_payment. Errors on a rate at or below -100 or negative periods.",
            "today:f = danger(finance_present_value(1628.89, 5.0, 10));";
        "finance_future_value" => "std_lib::finance::future_value", (present_value: f, rate_percent: f, periods: i) -> (f!e),
            "Returns what a present amount grows to at the yearly rate over the given periods, compounded once per period. The rate is the yearly percentage as people quote it, 6.0 means six percent, matching money_loan_payment. Errors on a rate at or below -100 or negative periods.",
            "grown:f = danger(finance_future_value(1000.0, 5.0, 10));";
        "finance_npv" => "std_lib::finance::npv", (rate_percent: f, cash_flows: (&[f])) -> (f!e),
            "Returns the net present value of the flows at the yearly discount rate in percent. The first flow sits at time zero undiscounted and each later flow is discounted one period more, the way a flow list starting with the up front investment reads. Errors on an empty array or a rate at or below -100.",
            "value:f = danger(finance_npv(10.0, flows));";
        "finance_irr" => "std_lib::finance::irr", (cash_flows: (&[f])) -> (f!e),
            "Returns the internal rate of return in percent, the discount rate at which finance_npv of the flows is zero. Newton's method from ten percent with a bisection fallback over a scan from -99 to 10000 percent. Errors when the flows never change sign or when no rate in that range works.",
            "rate:f = danger(finance_irr(flows));";
        "finance_cagr" => "std_lib::finance::cagr", (beginning: f, ending: f, periods: i) -> (f!e),
            "Returns the compound annual growth rate in percent, the steady yearly rate carrying the beginning value to the ending value over the periods. Errors unless both values are positive and there is at least one period.",
            "growth:f = danger(finance_cagr(1000.0, 2000.0, 10));";
        "finance_roi_percent" => "std_lib::finance::roi_percent", (cost: f, gain: f) -> (f!e),
            "Returns the return on investment in percent, the gain minus the cost as a share of the cost. Errors on a zero cost.",
            "roi:f = danger(finance_roi_percent(1000.0, 1500.0));";
        "finance_payback_periods" => "std_lib::finance::payback_periods", (cash_flows: (&[f])) -> (f!e),
            "Returns how many periods the flows take to pay back the up front investment, interpolated linearly inside the period where the running total crosses zero. The first flow must be negative. Errors when the investment never pays back.",
            "periods:f = danger(finance_payback_periods(flows));";
        "finance_compound" => "std_lib::finance::compound", (principal: f, rate_percent: f, compounds_per_year: i, years: f) -> (f!e),
            "Returns what a principal grows to under compound interest, with the yearly rate in percent compounded the given number of times a year for the given years. The rate is the yearly percentage as people quote it, 6.0 means six percent, matching money_loan_payment. Errors outside 1 to 366 compounds, on a rate at or below -100 or on negative years.",
            "balance:f = danger(finance_compound(1000.0, 6.0, 12, 10.0));";
        "finance_effective_rate" => "std_lib::finance::effective_rate", (nominal_percent: f, compounds_per_year: i) -> (f!e),
            "Returns the effective yearly rate in percent, what a nominal yearly rate quoted with compounding actually earns in a year, the APY behind an APR. Errors outside 1 to 366 compounds or on a nominal rate at or below -100.",
            "apy:f = danger(finance_effective_rate(6.0, 12));";
        "finance_rule_of_72_years" => "std_lib::finance::rule_of_72_years", (rate_percent: f) -> (f!e),
            "Returns the rule of 72 estimate of how many years money takes to double at the yearly rate in percent, 72 divided by the rate. Errors unless the rate is positive.",
            "doubling:f = danger(finance_rule_of_72_years(8.0));";
    }
}
