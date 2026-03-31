export interface ObservedMeta {
  name: string;
  loc: string;
  type: 'var' | 'dep' | 'props' | 'attr';
}

export interface ComponentMetaData {
  id: string;
  name?: string;
  code: string;
  loc?: string;
  observedList: ObservedMeta[];
  shouldDetectCallStack: boolean;
}

export interface ImportMeta {
  source: string;
  specifiers: string[];
}

export interface SwcAnalyzeResult {
  ok: boolean;
  imports: ImportMeta[];
  components: ComponentMetaData[];
  errors: string[];
}

// eslint-disable-next-line @typescript-eslint/no-var-requires
const wasmBinding = require('../dist/wasm/relyzer_swc.js');

export async function analyzeWithSwc(code: string): Promise<SwcAnalyzeResult> {
  if (typeof wasmBinding?.analyze !== 'function') {
    return {
      ok: false,
      imports: [],
      components: [],
      errors: ['SWC Wasm analyze export is not available in the current runtime.'],
    };
  }

  const raw = wasmBinding.analyze(code);
  return JSON.parse(raw) as SwcAnalyzeResult;
}
