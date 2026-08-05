//! The time value of money: what a future amount is worth today, what an
//! investment really earns per year, when a project pays for itself. Plain
//! float arithmetic for planning and comparison, with the money module
//! handling cent-exact bookkeeping. Every rate here is the yearly percentage
//! as people quote it, so 6.0 means six percent, the same convention as
//! money_loan_payment.

/// Shared guard, so every function refuses a rate at or below -100 percent
/// the same way. Losing everything in a period leaves nothing to grow or
/// discount.
fn require_rate_above_minus_100(function: &str, rate_percent: f64) -> Result<(), String> {
    if !(rate_percent > -100.0) {
        return Err(format!("{}: a rate of {} percent means losing everything or more, the rate must be above -100", function, rate_percent));
    }
    return Ok(());
}

/// Shared guard for compounding counts, on the same bounds the money module
/// holds lenders to.
fn require_compounds(function: &str, compounds_per_year: i64) -> Result<(), String> {
    if !(1..=366).contains(&compounds_per_year) {
        return Err(format!("{}: {} compounds a year is not how anyone counts, use 1 to 366", function, compounds_per_year));
    }
    return Ok(());
}

/// Shared guard for period counts, since the clock here only runs forward.
fn require_periods_not_negative(function: &str, periods: i64) -> Result<(), String> {
    if periods < 0 {
        return Err(format!("{}: {} periods is negative, and the clock here only runs forward", function, periods));
    }
    return Ok(());
}

/// What a future amount is worth today, discounted at the yearly rate over
/// the given number of periods. The reverse of finance_future_value. The
/// rate is the yearly percentage as people quote it, so 6.0 means six
/// percent.
pub fn present_value(future_value: f64, rate_percent: f64, periods: i64) -> Result<f64, String> {
    require_rate_above_minus_100("finance_present_value", rate_percent)?;
    require_periods_not_negative("finance_present_value", periods)?;
    let growth = 1.0 + rate_percent / 100.0;
    return Ok(future_value / growth.powf(periods as f64));
}

/// What a present amount grows to at the yearly rate over the given number
/// of periods, compounded once per period. The rate is the yearly percentage
/// as people quote it, so 6.0 means six percent, the same convention as
/// money_loan_payment.
pub fn future_value(present_value: f64, rate_percent: f64, periods: i64) -> Result<f64, String> {
    require_rate_above_minus_100("finance_future_value", rate_percent)?;
    require_periods_not_negative("finance_future_value", periods)?;
    let growth = 1.0 + rate_percent / 100.0;
    return Ok(present_value * growth.powf(periods as f64));
}

/// The net present value at a rate the caller has already checked, kept
/// private so finance_irr can probe rates without re-checking every time.
fn npv_at(rate_percent: f64, cash_flows: &Vec<f64>) -> f64 {
    let growth = 1.0 + rate_percent / 100.0;
    let mut total = 0.0;
    let mut discount = 1.0;
    for flow in cash_flows.iter() {
        total += flow / discount;
        discount *= growth;
    }
    return total;
}

/// How fast the net present value moves as the rate rises one percentage
/// point, for Newton's method inside finance_irr. The time zero flow never
/// moves, so it contributes nothing here.
fn npv_slope(rate_percent: f64, cash_flows: &Vec<f64>) -> f64 {
    let growth = 1.0 + rate_percent / 100.0;
    let mut slope = 0.0;
    for (time, flow) in cash_flows.iter().enumerate().skip(1) {
        slope -= flow * time as f64 / (100.0 * growth.powf(time as f64 + 1.0));
    }
    return slope;
}

/// The net present value of a series of cash flows at the given yearly
/// discount rate in percent. The first flow lands at time zero and is not
/// discounted, and each later flow is discounted one period more. That is
/// how a flow list starting with the up front investment reads, the
/// convention XNPV follows, where Excel's plain NPV would discount even the
/// first flow.
pub fn npv(rate_percent: f64, cash_flows: &Vec<f64>) -> Result<f64, String> {
    require_rate_above_minus_100("finance_npv", rate_percent)?;
    if cash_flows.is_empty() {
        return Err("finance_npv: the array is empty, so there are no flows to value".to_string());
    }
    return Ok(npv_at(rate_percent, cash_flows));
}

/// The internal rate of return in percent, the discount rate at which the
/// net present value of the flows is exactly zero. Newton's method starts
/// from ten percent, and when it wanders off a bracket scan from -99 to
/// 10000 percent hands the root to bisection. The flows must include at
/// least one negative and one positive value, since flows that never change
/// sign keep the same sign of value at every rate.
pub fn irr(cash_flows: &Vec<f64>) -> Result<f64, String> {
    if cash_flows.is_empty() {
        return Err("finance_irr: the array is empty, so there are no flows to find a rate for".to_string());
    }
    let has_negative = cash_flows.iter().any(|flow| *flow < 0.0);
    let has_positive = cash_flows.iter().any(|flow| *flow > 0.0);
    if !has_negative || !has_positive {
        return Err("finance_irr: the flows never change sign, so no rate can bring their value to zero".to_string());
    }

    let scale = cash_flows.iter().fold(1.0_f64, |biggest, flow| biggest.max(flow.abs()));
    let tolerance = scale * 1e-10;

    let mut rate = 10.0;
    for _ in 0..100 {
        let value = npv_at(rate, cash_flows);
        if value.abs() <= tolerance {
            return Ok(rate);
        }
        let slope = npv_slope(rate, cash_flows);
        if slope == 0.0 || !slope.is_finite() {
            break;
        }
        let next = rate - value / slope;
        if !next.is_finite() || next <= -100.0 {
            break;
        }
        rate = next;
    }
    if npv_at(rate, cash_flows).abs() <= tolerance {
        return Ok(rate);
    }

    let mut previous_rate = -99.0;
    let mut previous_value = npv_at(previous_rate, cash_flows);
    if previous_value == 0.0 {
        return Ok(previous_rate);
    }
    for step in -98..=10000_i64 {
        let scan_rate = step as f64;
        let scan_value = npv_at(scan_rate, cash_flows);
        if scan_value == 0.0 {
            return Ok(scan_rate);
        }
        if (previous_value < 0.0) != (scan_value < 0.0) {
            let mut low = previous_rate;
            let mut high = scan_rate;
            let low_negative = previous_value < 0.0;
            for _ in 0..200 {
                let middle = (low + high) / 2.0;
                let middle_value = npv_at(middle, cash_flows);
                if middle_value.abs() <= tolerance {
                    return Ok(middle);
                }
                if (middle_value < 0.0) == low_negative {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            return Ok((low + high) / 2.0);
        }
        previous_rate = scan_rate;
        previous_value = scan_value;
    }
    return Err("finance_irr: no rate between -99 and 10000 percent brings these flows to zero".to_string());
}

/// The compound annual growth rate in percent, the steady yearly rate that
/// carries the beginning value to the ending value over the given number of
/// periods. Negative when the ending value is smaller, which is a perfectly
/// honest growth rate.
pub fn cagr(beginning: f64, ending: f64, periods: i64) -> Result<f64, String> {
    if !(beginning > 0.0) {
        return Err(format!("finance_cagr: the beginning value is {} and must be positive to grow from", beginning));
    }
    if !(ending > 0.0) {
        return Err(format!("finance_cagr: the ending value is {} and must be positive to grow to", ending));
    }
    if periods < 1 {
        return Err(format!("finance_cagr: {} periods cannot spread out growth, at least 1 is needed", periods));
    }
    return Ok(((ending / beginning).powf(1.0 / periods as f64) - 1.0) * 100.0);
}

/// The plain return on investment in percent, what came back minus what it
/// cost as a share of what it cost. A 1000 cost returning 1500 is a 50
/// percent return.
pub fn roi_percent(cost: f64, gain: f64) -> Result<f64, String> {
    if cost == 0.0 {
        return Err("finance_roi_percent: the cost is zero, so the return cannot be measured as a share of it".to_string());
    }
    return Ok((gain - cost) / cost * 100.0);
}

/// How many periods until the running total of the flows first climbs back
/// to zero, with the answer interpolated linearly inside the period where
/// the crossing happens. The first flow is the up front investment and must
/// be negative, and the later flows are what trickles back in.
pub fn payback_periods(cash_flows: &Vec<f64>) -> Result<f64, String> {
    if cash_flows.is_empty() {
        return Err("finance_payback_periods: the array is empty, so there is nothing to pay back".to_string());
    }
    if !(cash_flows[0] < 0.0) {
        return Err(format!("finance_payback_periods: the first flow is {} and must be negative, the investment the later flows pay back", cash_flows[0]));
    }
    let mut cumulative = cash_flows[0];
    for (time, flow) in cash_flows.iter().enumerate().skip(1) {
        let next_cumulative = cumulative + flow;
        // Reaching here means cumulative is still below zero, so a crossing
        // flow is large enough to be safely divided by.
        if next_cumulative >= 0.0 {
            return Ok((time as f64 - 1.0) + (-cumulative) / flow);
        }
        cumulative = next_cumulative;
    }
    return Err("finance_payback_periods: the flows never climb back to zero, so the investment never pays back".to_string());
}

/// What a principal grows to under compound interest, the classic
/// A = P(1 + r/m)^(mt) with the yearly rate in percent split across m
/// compounding dates a year for t years. 12 compounds a year is the
/// everyday bank account, 1 matches finance_future_value over whole years,
/// and 365 approximates daily compounding. The rate is the yearly
/// percentage as people quote it, so 6.0 means six percent.
pub fn compound(principal: f64, rate_percent: f64, compounds_per_year: i64, years: f64) -> Result<f64, String> {
    require_rate_above_minus_100("finance_compound", rate_percent)?;
    require_compounds("finance_compound", compounds_per_year)?;
    if !(years >= 0.0) {
        return Err(format!("finance_compound: {} years is not a length of time to grow over, it cannot be negative", years));
    }
    let per_period = 1.0 + rate_percent / (100.0 * compounds_per_year as f64);
    return Ok(principal * per_period.powf(compounds_per_year as f64 * years));
}

/// The effective yearly rate in percent, what a nominal yearly rate quoted
/// with compounding actually earns over a full year once each period's
/// interest starts earning its own. The APY behind an APR: a 6 percent
/// nominal rate compounded monthly really earns about 6.17 percent.
pub fn effective_rate(nominal_percent: f64, compounds_per_year: i64) -> Result<f64, String> {
    require_rate_above_minus_100("finance_effective_rate", nominal_percent)?;
    require_compounds("finance_effective_rate", compounds_per_year)?;
    let per_period = 1.0 + nominal_percent / (100.0 * compounds_per_year as f64);
    return Ok((per_period.powf(compounds_per_year as f64) - 1.0) * 100.0);
}

/// The rule of 72 estimate of how many years money takes to double at the
/// given yearly rate in percent, 72 divided by the rate. A mental math
/// shortcut rather than an exact answer, good to a few months at everyday
/// rates.
pub fn rule_of_72_years(rate_percent: f64) -> Result<f64, String> {
    if !(rate_percent > 0.0) {
        return Err(format!("finance_rule_of_72_years: a rate of {} percent never doubles anything, the rate must be positive", rate_percent));
    }
    return Ok(72.0 / rate_percent);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        return (left - right).abs() < 1e-6;
    }

    #[test]
    fn a_thousand_at_five_percent_for_ten_years_is_the_spreadsheet_number() {
        assert!(close(future_value(1000.0, 5.0, 10).expect("grows"), 1628.894627));
    }

    #[test]
    fn present_value_inverts_future_value() {
        assert!(close(present_value(1628.894627, 5.0, 10).expect("discounts"), 1000.0));
        let grown = future_value(250.0, 7.25, 30).expect("grows");
        assert!(close(present_value(grown, 7.25, 30).expect("discounts"), 250.0));
    }

    #[test]
    fn zero_periods_leave_an_amount_untouched() {
        assert!(close(future_value(500.0, 5.0, 0).expect("grows"), 500.0));
        assert!(close(present_value(500.0, 5.0, 0).expect("discounts"), 500.0));
    }

    #[test]
    fn npv_leaves_the_first_flow_undiscounted() {
        let flows = vec![-1000.0, 300.0, 300.0, 300.0, 300.0];
        assert!(close(npv(10.0, &flows).expect("flows"), -49.040366095));
        // At a zero rate nothing is discounted, so the value is the plain sum.
        assert!(close(npv(0.0, &flows).expect("flows"), 200.0));
    }

    #[test]
    fn irr_matches_excel_and_zeroes_the_npv() {
        let flows = vec![-1000.0, 300.0, 300.0, 300.0, 300.0];
        let rate = irr(&flows).expect("changes sign");
        assert!((rate - 7.713847).abs() < 1e-4);
        assert!(npv(rate, &flows).expect("flows").abs() < 1e-6);
    }

    #[test]
    fn irr_finds_a_deeply_negative_rate_through_the_fallback_scan() {
        // Newton's first step from ten percent shoots below -100 here, so
        // the bracket scan takes over and lands exactly on -50.
        assert_eq!(irr(&vec![-1000.0, 500.0]).expect("changes sign"), -50.0);
    }

    #[test]
    fn irr_names_flows_no_rate_can_balance() {
        // -100 + 210x - 200x squared stays negative for every discount
        // factor x, so this series changes sign yet has no rate of return.
        assert!(irr(&vec![-100.0, 210.0, -200.0]).unwrap_err().contains("no rate between"));
    }

    #[test]
    fn irr_demands_flows_in_both_directions() {
        assert!(irr(&vec![]).unwrap_err().contains("empty"));
        assert!(irr(&vec![100.0, 200.0]).unwrap_err().contains("never change sign"));
        assert!(irr(&vec![-100.0, -200.0]).unwrap_err().contains("never change sign"));
    }

    #[test]
    fn doubling_in_ten_periods_grows_seven_point_two_percent_a_year() {
        assert!(close(cagr(1000.0, 2000.0, 10).expect("grows"), 7.177346));
        assert!(close(cagr(2000.0, 1000.0, 10).expect("shrinks"), -6.696701));
    }

    #[test]
    fn cagr_needs_positive_values_and_at_least_one_period() {
        assert!(cagr(0.0, 2000.0, 10).unwrap_err().contains("beginning"));
        assert!(cagr(-5.0, 2000.0, 10).unwrap_err().contains("beginning"));
        assert!(cagr(1000.0, 0.0, 10).unwrap_err().contains("ending"));
        assert!(cagr(1000.0, 2000.0, 0).unwrap_err().contains("at least 1"));
    }

    #[test]
    fn roi_is_the_gain_over_the_cost_as_a_percent() {
        assert!(close(roi_percent(1000.0, 1500.0).expect("cost"), 50.0));
        assert!(close(roi_percent(2000.0, 1500.0).expect("cost"), -25.0));
        assert!(roi_percent(0.0, 100.0).unwrap_err().contains("zero"));
    }

    #[test]
    fn payback_lands_exactly_on_a_period_boundary() {
        assert_eq!(payback_periods(&vec![-1000.0, 500.0, 500.0, 500.0]).expect("pays back"), 2.0);
    }

    #[test]
    fn payback_interpolates_inside_the_crossing_period() {
        // Running totals -1000, -700, -300, 200 cross during the third
        // period, 300 of the 500 into it.
        assert!(close(payback_periods(&vec![-1000.0, 300.0, 400.0, 500.0]).expect("pays back"), 2.6));
        assert!(close(payback_periods(&vec![-100.0, 400.0]).expect("pays back"), 0.25));
    }

    #[test]
    fn payback_demands_an_investment_that_comes_back() {
        assert!(payback_periods(&vec![]).unwrap_err().contains("empty"));
        assert!(payback_periods(&vec![1000.0, 500.0]).unwrap_err().contains("must be negative"));
        assert!(payback_periods(&vec![-1000.0, 300.0, 300.0]).unwrap_err().contains("never pays back"));
    }

    #[test]
    fn compounding_monthly_matches_the_spreadsheet_and_beats_yearly() {
        assert!(close(compound(1000.0, 6.0, 12, 1.0).expect("grows"), 1061.677812));
        assert!(close(compound(1000.0, 5.0, 1, 10.0).expect("grows"), 1628.894627));
        assert!(compound(1000.0, 6.0, 12, 1.0).expect("grows") > compound(1000.0, 6.0, 1, 1.0).expect("grows"));
    }

    #[test]
    fn compound_checks_its_count_and_clock() {
        assert!(compound(1000.0, 5.0, 0, 1.0).unwrap_err().contains("use 1 to 366"));
        assert!(compound(1000.0, 5.0, 367, 1.0).unwrap_err().contains("use 1 to 366"));
        assert!(compound(1000.0, 5.0, 12, -1.0).unwrap_err().contains("negative"));
    }

    #[test]
    fn a_six_percent_apr_compounded_monthly_earns_six_point_one_seven() {
        assert!(close(effective_rate(6.0, 12).expect("compounds"), 6.167781));
        assert!(close(effective_rate(6.0, 1).expect("compounds"), 6.0));
    }

    #[test]
    fn effective_rate_checks_its_compounding_count() {
        assert!(effective_rate(6.0, 0).unwrap_err().contains("use 1 to 366"));
        assert!(effective_rate(6.0, 367).unwrap_err().contains("use 1 to 366"));
    }

    #[test]
    fn eight_percent_doubles_in_nine_years_by_the_rule_of_72() {
        assert_eq!(rule_of_72_years(8.0).expect("positive"), 9.0);
        assert_eq!(rule_of_72_years(6.0).expect("positive"), 12.0);
    }

    #[test]
    fn the_rule_of_72_needs_a_positive_rate() {
        assert!(rule_of_72_years(0.0).unwrap_err().contains("must be positive"));
        assert!(rule_of_72_years(-3.0).unwrap_err().contains("must be positive"));
    }

    #[test]
    fn every_rate_taker_refuses_minus_one_hundred_percent() {
        assert!(future_value(1000.0, -100.0, 5).unwrap_err().contains("above -100"));
        assert!(present_value(1000.0, -150.0, 5).unwrap_err().contains("above -100"));
        assert!(npv(-100.0, &vec![1.0]).unwrap_err().contains("above -100"));
        assert!(compound(1000.0, -100.0, 12, 1.0).unwrap_err().contains("above -100"));
        assert!(effective_rate(-100.0, 12).unwrap_err().contains("above -100"));
    }

    #[test]
    fn negative_periods_are_refused() {
        assert!(future_value(1000.0, 5.0, -1).unwrap_err().contains("negative"));
        assert!(present_value(1000.0, 5.0, -1).unwrap_err().contains("negative"));
    }

    #[test]
    fn npv_refuses_an_empty_array() {
        assert!(npv(10.0, &vec![]).unwrap_err().contains("empty"));
    }
}
