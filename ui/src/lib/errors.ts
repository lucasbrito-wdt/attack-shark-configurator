// ApiError -> interface copy. `message` from the backend is for logs; people read these instead.

import { t } from "../i18n";
import { errorCode } from "./caps";

export function errorText(e: unknown, model?: string): string {
  const m = model ?? t("errors.thisModel");
  switch (errorCode(e)) {
    case "unverified_encoding":
      return t("errors.unverifiedEncoding", { model: m });
    case "unsupported_model":
      return t("errors.unsupportedModel");
    case "invalid_input":
      return t("errors.invalidInput");
    case "not_found":
      return t("errors.notFound");
    case "last_profile":
      return t("errors.lastProfile");
    case "busy":
      return t("errors.busy");
    case "unsupported":
      return t("errors.unsupported");
    case "timeout":
      return t("errors.timeout");
    case "decode_failed":
      return t("errors.decodeFailed", { model: m });
    case "confirm_required":
      return t("errors.confirmRequired");
    default:
      return t("errors.noAnswer");
  }
}
