use crate::model_updates::CatalogModel;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionCost {
    pub estimated_usd: f64,
    pub usd_per_minute: f64,
    pub pricing_checked_at: String,
    pub billing: String,
}
pub fn estimate(model: Option<&CatalogModel>, seconds: f64) -> Option<TranscriptionCost> {
    let model = model.filter(|m| m.valid())?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some(TranscriptionCost {
        estimated_usd: seconds / 60.0 * model.estimated_usd_per_minute,
        usd_per_minute: model.estimated_usd_per_minute,
        pricing_checked_at: model.pricing_checked_at.clone(),
        billing: model.billing.clone(),
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn short_recording_preserves_small_cost_and_price_snapshot() {
        let mut model = crate::model_updates::default_model();
        let saved = super::estimate(Some(&model), 30.0).unwrap();
        assert!((saved.estimated_usd - 0.00225).abs() < 1e-12);
        model.estimated_usd_per_minute = 0.1;
        assert_eq!(saved.usd_per_minute, 0.0045);
        assert!(super::estimate(Some(&model), f64::NAN).is_none());
        assert!(super::estimate(None, 30.0).is_none());
    }
}
