import type { Prompt, PromptVersion } from "../../types";
import { EvaluationWorkbench } from "../../../evaluation/EvaluationWorkbench";

export interface RunTabProps {
  prompt: Prompt;
  versions: PromptVersion[];
}

export function RunTab({ prompt, versions }: RunTabProps) {
  return <EvaluationWorkbench prompt={prompt} versions={versions} />;
}
