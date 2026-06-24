import { ArrowLeft, ArrowRight, Box, FolderOpen, Languages, PlugZap } from "lucide-react";

import { languages, type Language } from "../i18n";
import { canFinishOnboardingWithPrism, type PrismRootState } from "../onboarding";
import type { PrismRootValidation } from "../tauri";
import { PromptBlock, StatusRows, StepIcon } from "./settingsControls";
import type { Translator } from "./types";

export function OnboardingScreen(props: {
  language: Language;
  onBack: () => void;
  onChoosePrismRoot: () => void;
  onFinish: () => void;
  onLanguageChange: (language: Language) => void;
  onNextAi: () => void;
  onNextLanguage: () => void;
  onSkip: () => void;
  prismRoot: PrismRootState;
  prismValidation: PrismRootValidation | null;
  step: "language" | "ai" | "prism";
  t: Translator;
}) {
  const { t } = props;
  return (
    <main className="onboarding-shell">
      <section className="onboarding-panel" aria-label={t("onboarding.title")}>
        <div className="brand onboarding-brand">
          <div className="brand-mark">
            <Box size={18} />
          </div>
          <div>
            <h1>{t("app.title")}</h1>
            <span>{t("onboarding.title")}</span>
          </div>
        </div>

        {props.step === "language" && (
          <div className="onboarding-step">
            <StepIcon>
              <Languages size={22} />
            </StepIcon>
            <h2>{t("onboarding.languageTitle")}</h2>
            <p>{t("onboarding.languageBody")}</p>
            <div className="choice-row">
              {languages.map((option) => (
                <button
                  className={option === props.language ? "choice-button active" : "choice-button"}
                  key={option}
                  onClick={() => props.onLanguageChange(option)}
                  type="button"
                >
                  {option.toUpperCase()}
                </button>
              ))}
            </div>
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onSkip} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="primary-action compact" onClick={props.onNextLanguage} type="button">
                  {t("onboarding.next")}
                  <ArrowRight size={16} />
                </button>
              </div>
            </div>
          </div>
        )}

        {props.step === "ai" && (
          <div className="onboarding-step">
            <StepIcon>
              <PlugZap size={22} />
            </StepIcon>
            <h2>{t("onboarding.aiTitle")}</h2>
            <p>{t("onboarding.aiBody")}</p>
            <PromptBlock endpoint={null} language={props.language} t={t} />
            <StatusRows
              rows={[
                [t("settings.status"), t("status.aiDisconnected")],
                [t("settings.activeClient"), t("settings.noActiveClient")],
              ]}
            />
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onSkip} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="secondary-action compact" onClick={props.onBack} type="button">
                  <ArrowLeft size={16} />
                  {t("onboarding.back")}
                </button>
                <button className="primary-action compact" onClick={props.onNextAi} type="button">
                  {t("onboarding.next")}
                  <ArrowRight size={16} />
                </button>
              </div>
            </div>
          </div>
        )}

        {props.step === "prism" && (
          <div className="onboarding-step">
            <StepIcon>
              <FolderOpen size={22} />
            </StepIcon>
            <h2>{t("onboarding.prismTitle")}</h2>
            <p>{t("onboarding.prismBody")}</p>
            <p className="subtle-line">{t("settings.prismFolderHint")}</p>
            <StatusRows
              rows={[
                [t("settings.status"), props.prismValidation?.message ?? t("settings.prismDetecting")],
                [
                  t("settings.prismInstances"),
                  props.prismValidation ? String(props.prismValidation.instanceCount) : t("library.unknown"),
                ],
              ]}
            />
            <button className="secondary-action compact" onClick={props.onChoosePrismRoot} type="button">
              <FolderOpen size={16} />
              {t("settings.choosePrismRoot")}
            </button>
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onSkip} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="secondary-action compact" onClick={props.onBack} type="button">
                  <ArrowLeft size={16} />
                  {t("onboarding.back")}
                </button>
                <button
                  className="primary-action compact"
                  disabled={!canFinishOnboardingWithPrism(props.prismRoot)}
                  onClick={props.onFinish}
                  type="button"
                >
                  {t("onboarding.finish")}
                </button>
              </div>
            </div>
          </div>
        )}
      </section>
    </main>
  );
}
