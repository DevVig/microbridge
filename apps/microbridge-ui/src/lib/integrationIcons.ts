import chatgpt from "../assets/integrations/chatgpt.png";
import claude from "../assets/integrations/claude.png";
import cnvs from "../assets/integrations/cnvs.png";
import codex from "../assets/integrations/codex.png";
import conductor from "../assets/integrations/conductor.png";
import cursor from "../assets/integrations/cursor.png";
import factory from "../assets/integrations/factory.png";
import opencode from "../assets/integrations/opencode.png";
import synara from "../assets/integrations/synara.png";
import t3code from "../assets/integrations/t3code.png";

/** App icons keyed by daemon adapter id. */
export const INTEGRATION_ICONS: Record<string, string> = {
  chatgpt,
  claude,
  claude_desktop: claude,
  cnvs,
  codex,
  conductor,
  cursor,
  cursor_acp: cursor,
  factory,
  opencode,
  synara,
  t3code,
};

export function integrationIcon(adapterId: string): string | undefined {
  return INTEGRATION_ICONS[adapterId];
}
