import type { Prompt, PromptTypeDefinition, PromptVersion } from "../../types";
import { VersionHistory } from "../VersionHistory";

export interface VersionTabProps {
  prompt: Prompt;
  versions: PromptVersion[];
  promptTypeDefinitions: PromptTypeDefinition[];
  onCreateVersion: (note?: string) => void;
  onRollback: (version: number) => void;
}

export function VersionTab(props: VersionTabProps) {
  return <VersionHistory {...props} />;
}
