import type { PromptMessage } from "../prompts/types";

export interface ExecutionProfileInput {
  profileId?: string | null;
  name: string;
  provider: "mock" | "openai-compatible";
  endpoint?: string | null;
  model: string;
  parameters: Record<string, unknown>;
  credential?: string | null;
}

export interface ExecutionProfileRevision {
  id: string;
  profileId: string;
  revision: number;
  name: string;
  provider: "mock" | "openai-compatible";
  endpoint?: string | null;
  model: string;
  parameters: Record<string, unknown>;
  hasCredential: boolean;
  createdAt: string;
}

export interface RenderedPrompt {
  promptRevisionId: string;
  messages: PromptMessage[];
}

export interface PromptRunInput {
  promptRevisionId: string;
  profileRevisionId: string;
  inputs: Record<string, string>;
  testCaseId?: string | null;
}

export interface PromptRun {
  id: string;
  promptRevisionId: string;
  profileRevisionId: string;
  testCaseId?: string | null;
  inputs: Record<string, string>;
  renderedMessages: PromptMessage[];
  output?: string | null;
  status: "running" | "success" | "error" | "cancelled";
  error?: string | null;
  startedAt: string;
  completedAt?: string | null;
  durationMs?: number | null;
  usage?: Record<string, unknown> | null;
  cacheKey?: string | null;
}

export interface TestCaseInput {
  id?: string | null;
  name: string;
  inputs: Record<string, string>;
  expectedOutput?: string | null;
  annotations: Record<string, unknown>;
}

export interface TestSetInput {
  id?: string | null;
  name: string;
  cases: TestCaseInput[];
}

export interface TestCase {
  id: string;
  name: string;
  inputs: Record<string, string>;
  expectedOutput?: string | null;
  annotations: Record<string, unknown>;
  sortOrder: number;
}

export interface TestSet {
  id: string;
  name: string;
  cases: TestCase[];
  createdAt: string;
  updatedAt: string;
}

export interface EvaluatorInput {
  name: string;
  kind: "manual" | "exact" | "contains" | "regex" | "numeric";
  config: Record<string, unknown>;
}

export interface EvaluatorConfig extends EvaluatorInput {
  id: string;
  createdAt: string;
}

export interface EvaluationResult {
  evaluatorId: string;
  kind: EvaluatorInput["kind"];
  passed?: boolean | null;
  score?: number | null;
  skipped: boolean;
  evidence: string;
}

export interface EvaluationMatrixInput {
  testSetId: string;
  promptRevisionIds: string[];
  profileRevisionIds: string[];
  evaluatorIds: string[];
}

export interface EvaluationRun {
  id: string;
  testSetId: string;
  promptRevisionIds: string[];
  profileRevisionIds: string[];
  evaluatorIds: string[];
  status: PromptRun["status"];
  totalCells: number;
  completedCells: number;
  failedCells: number;
  startedAt: string;
  completedAt?: string | null;
  runtimeVersion: string;
}

export interface EvaluationCell {
  id: string;
  evaluationRunId: string;
  promptRevisionId: string;
  profileRevisionId: string;
  testCaseId: string;
  promptRunId?: string | null;
  status: "pending" | "running" | "success" | "error" | "cancelled" | "skipped";
  cacheHit: boolean;
  results: EvaluationResult[];
  error?: string | null;
  cacheKey: string;
  sortOrder: number;
}

export interface EvaluationRunDetail {
  run: EvaluationRun;
  cells: EvaluationCell[];
}

export interface PromptLabel {
  promptId: string;
  label: "baseline" | "candidate";
  promptRevisionId: string;
  updatedAt: string;
}

export interface PromptLabelHistory {
  id: string;
  promptId: string;
  label: PromptLabel["label"];
  fromRevisionId?: string | null;
  toRevisionId: string;
  action: "move" | "rollback";
  createdAt: string;
}

export interface RunChunkEvent {
  runId: string;
  chunk: string;
}

export interface MatrixProgressEvent {
  evaluationRunId: string;
  completed: number;
  total: number;
  cellId: string;
}
