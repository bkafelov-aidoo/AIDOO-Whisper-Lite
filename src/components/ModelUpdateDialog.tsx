import { ArrowRight, LoaderCircle, X } from "lucide-react";
import { useDialogFocus } from "../hooks/useDialogFocus";
import { translator } from "../i18n";
import type { AppLanguage, ModelProposal } from "../types";

export function ModelUpdateDialog({ proposal, language, saving, onAccept, onLater }: {
  proposal: ModelProposal; language: AppLanguage; saving: boolean;
  onAccept: () => void; onLater: () => void;
}) {
  const t = translator(language);
  const dialogRef = useDialogFocus(onLater, !saving);
  const price = (amount: number) => new Intl.NumberFormat(language === "bg" ? "bg-BG" : "en-US", { style: "currency", currency: "USD", minimumFractionDigits: 4, maximumFractionDigits: 4 }).format(amount);
  return <div className="modal-backdrop small"><section ref={dialogRef} className="confirm-dialog model-update-dialog" role="dialog" aria-modal="true" aria-labelledby="model-update-title" aria-describedby="model-update-benefit" tabIndex={-1}>
    <button className="close-button" aria-label={t("modelLater")} disabled={saving} onClick={onLater}><X /></button>
    <h2 id="model-update-title">{t("modelUpdateTitle")}</h2>
    <div className="model-update-comparison"><span>{proposal.currentModel}</span><ArrowRight aria-hidden="true" /><strong>{proposal.candidate.id}</strong></div>
    <p id="model-update-benefit">{language === "bg" ? proposal.candidate.benefitBg : proposal.candidate.benefitEn}</p>
    <div className="model-update-price"><span>{t("modelCurrentPrice")}: {price(proposal.currentUsdPerMinute)}</span><strong>{t("modelNewPrice")}: {price(proposal.candidate.estimatedUsdPerMinute)}</strong><small>{t("modelPriceEstimate")}</small></div>
    <p>{t("modelUpdateScope")}</p>
    <button className="primary-button" disabled={saving} onClick={onAccept}>{saving && <LoaderCircle className="spin" />}{t("modelUse")}</button>
    <button className="secondary-button" disabled={saving} onClick={onLater}>{t("modelLater")}</button>
  </section></div>;
}
