import type { AppSettings, CatalogModel } from "../types";

// Catalog availability belongs to the current account, never to persisted model metadata.
export function modelDetailsForSettings(model: CatalogModel): CatalogModel {
  return {
    id: model.id, rank: model.rank, protocol: model.protocol,
    estimatedUsdPerMinute: model.estimatedUsdPerMinute,
    billing: model.billing, pricingCheckedAt: model.pricingCheckedAt,
    languages: [...model.languages], benefitBg: model.benefitBg,
    benefitEn: model.benefitEn, source: model.source,
  };
}

export function settingsForCommand(settings: AppSettings): AppSettings {
  return { ...settings, modelDetails: settings.modelDetails ? modelDetailsForSettings(settings.modelDetails) : null };
}
