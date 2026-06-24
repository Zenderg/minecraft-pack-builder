import { ArrowLeft, ArrowRight, Box, KeyRound, Languages, PlugZap } from "lucide-react";

import { languages, type Language } from "../i18n";
import { canFinishOnboardingWithKey, type CurseForgeKeyCheckResult, type CurseForgeKeyState } from "../onboarding";
import type { CurseForgeCredentialStatus } from "../tauri";
import { KeyForm, PromptBlock, StatusRows, StepIcon } from "./settingsControls";
import type { Translator } from "./types";

export function OnboardingScreen(props: {
  apiKeyInput: string;
  isSavingKey: boolean;
  keyState: CurseForgeKeyState;
  keyStatus: CurseForgeCredentialStatus | null;
  keyCheckResult: CurseForgeKeyCheckResult;
  keyCheckMessage: string;
  keyNotice: "idle" | "missing" | "saved" | "replaced" | "unavailable";
  language: Language;
  onBack: () => void;
  onCheckKey: () => void;
  onFinish: () => void;
  onLanguageChange: (language: Language) => void;
  onNextAi: () => void;
  onNextLanguage: () => void;
  onSkip: () => void;
  onUpdateKey: (value: string) => void;
  step: "language" | "ai" | "curseforge";
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

        {props.step === "curseforge" && (
          <div className="onboarding-step">
            <StepIcon>
              <KeyRound size={22} />
            </StepIcon>
            <h2>{t("onboarding.keyTitle")}</h2>
            <p>{t("onboarding.keyBody")}</p>
            <KeyForm
              apiKeyInput={props.apiKeyInput}
              isSavingKey={props.isSavingKey}
              keyCheckResult={props.keyCheckResult}
              keyCheckMessage={props.keyCheckMessage}
              keyNotice={props.keyNotice}
              keyState={props.keyState}
              keyStatus={props.keyStatus}
              onCheckKey={props.onCheckKey}
              onUpdateKey={props.onUpdateKey}
              t={t}
            />
            <div className="onboarding-actions split">
              <button className="ghost-action" onClick={props.onFinish} type="button">
                {t("onboarding.skip")}
              </button>
              <div className="nav-actions">
                <button className="secondary-action compact" onClick={props.onBack} type="button">
                  <ArrowLeft size={16} />
                  {t("onboarding.back")}
                </button>
                <button
                  className="primary-action compact"
                  disabled={!canFinishOnboardingWithKey(props.keyState)}
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

