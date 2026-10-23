"use client";

import { useEffect, useRef, useState } from "react";
import type { SuggestedFix } from "@/lib/types";

export interface SuggestedFixesProps {
  fixes: SuggestedFix[];
}

type CopyState = "idle" | "copied" | "error";

const DIFFICULTY_STYLES: Record<string, string> = {
  easy: "bg-emerald-950/40 border-emerald-900/60 text-emerald-300",
  medium: "bg-amber-950/40 border-amber-900/60 text-amber-300",
  hard: "bg-red-950/40 border-red-900/60 text-red-300",
};

const RUST_KEYWORDS = new Set([
  "as",
  "async",
  "await",
  "break",
  "const",
  "continue",
  "crate",
  "dyn",
  "else",
  "enum",
  "extern",
  "false",
  "fn",
  "for",
  "if",
  "impl",
  "in",
  "let",
  "loop",
  "match",
  "mod",
  "move",
  "mut",
  "pub",
  "ref",
  "return",
  "self",
  "Self",
  "static",
  "struct",
  "super",
  "trait",
  "true",
  "type",
  "unsafe",
  "use",
  "where",
  "while",
]);

// Ordered alternation: comments, strings, attributes, numbers, identifiers, whitespace, punctuation.
// Every character is covered so tokenizing never drops source text.
const TOKEN_PATTERN =
  /(\/\/[^\n]*|\/\*[\s\S]*?\*\/)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')|(#\s*!?\s*\[[^\]]*\])|(\b\d[\w.]*\b)|([A-Za-z_]\w*!?)|(\s+)|([^\s\w])/g;

const TOKEN_CLASSES: Record<number, string> = {
  1: "text-slate-500 italic",
  2: "text-emerald-300",
  3: "text-fuchsia-300",
  4: "text-amber-300",
  6: "",
  7: "text-slate-400",
};

interface CodeToken {
  text: string;
  className: string;
}

function tokenize(code: string): CodeToken[] {
  // Fresh lastIndex per call: tokenizing must never share state between blocks.
  const pattern = new RegExp(TOKEN_PATTERN.source, "g");
  const tokens: CodeToken[] = [];
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(code)) !== null) {
    let className = "";
    const identifier = match[5];

    if (identifier !== undefined) {
      if (identifier.endsWith("!")) {
        className = "text-sky-300";
      } else if (RUST_KEYWORDS.has(identifier)) {
        className = "text-violet-300";
      } else if (/^[A-Z]/.test(identifier)) {
        className = "text-teal-300";
      } else if (code[pattern.lastIndex] === "(") {
        className = "text-blue-300";
      }
    } else {
      for (const group of [1, 2, 3, 4, 7]) {
        if (match[group] !== undefined) {
          className = TOKEN_CLASSES[group];
          break;
        }
      }
    }

    tokens.push({ text: match[0], className });
  }

  return tokens;
}

async function copyToClipboard(text: string): Promise<boolean> {
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      // Clipboard API needs a secure context and permission; fall back below.
    }
  }

  if (typeof document === "undefined") {
    return false;
  }

  try {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const copied = document.execCommand("copy");
    document.body.removeChild(textarea);
    return copied;
  } catch {
    return false;
  }
}

function getDifficultyStyles(difficulty: string): string {
  return (
    DIFFICULTY_STYLES[difficulty.toLowerCase()] ??
    "bg-slate-950/60 border-slate-800 text-slate-400"
  );
}

function CodeBlock({ label, code }: { label: string; code: string }) {
  const [copyState, setCopyState] = useState<CopyState>("idle");
  const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (resetTimer.current) {
        clearTimeout(resetTimer.current);
      }
    };
  }, []);

  const handleCopy = async () => {
    const copied = await copyToClipboard(code);
    setCopyState(copied ? "copied" : "error");

    if (resetTimer.current) {
      clearTimeout(resetTimer.current);
    }
    resetTimer.current = setTimeout(() => setCopyState("idle"), 2000);
  };

  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950 overflow-hidden">
      <div className="flex items-center justify-between gap-2 border-b border-slate-800 bg-slate-900/70 px-3 py-1.5">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-500">
          {label}
        </span>
        <button
          type="button"
          onClick={handleCopy}
          aria-label={`Copy ${label.toLowerCase()} to clipboard`}
          className={`text-[10px] font-semibold uppercase tracking-wider px-2 py-0.5 rounded border transition-colors focus:outline-none focus:ring-2 focus:ring-violet-500 focus:ring-offset-1 focus:ring-offset-slate-950 ${
            copyState === "copied"
              ? "border-emerald-900/60 bg-emerald-950/40 text-emerald-300"
              : copyState === "error"
                ? "border-red-900/60 bg-red-950/40 text-red-300"
                : "border-slate-800 bg-slate-950 text-slate-400 hover:text-slate-200 hover:border-slate-700"
          }`}
        >
          {copyState === "copied"
            ? "✓ Copied"
            : copyState === "error"
              ? "Copy failed"
              : "Copy"}
        </button>
        <span role="status" aria-live="polite" className="sr-only">
          {copyState === "copied"
            ? `${label} copied to clipboard`
            : copyState === "error"
              ? `${label} could not be copied`
              : ""}
        </span>
      </div>
      <pre className="overflow-x-auto p-3 text-xs leading-relaxed font-mono text-slate-200">
        <code>
          {tokenize(code).map((token, i) => (
            <span key={i} className={token.className}>
              {token.text}
            </span>
          ))}
        </code>
      </pre>
    </div>
  );
}

export default function SuggestedFixes({ fixes }: SuggestedFixesProps) {
  return (
    <section
      aria-label="Suggested fixes"
      className="mt-6 border-t border-slate-800 pt-6"
    >
      <div className="flex items-center justify-between gap-2 mb-3">
        <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider">
          Suggested Fixes
        </h4>
        {fixes.length > 0 && (
          <span className="text-[10px] font-mono uppercase tracking-wider text-slate-600">
            {fixes.length} {fixes.length === 1 ? "remedy" : "remedies"}
          </span>
        )}
      </div>

      {fixes.length === 0 ? (
        <p className="text-sm text-slate-500 italic">
          No suggested fixes documented.
        </p>
      ) : (
        <ol className="space-y-3">
          {fixes.map((fix, index) => (
            <li
              key={fix.id || index}
              className="rounded-lg border border-slate-800 bg-slate-950/40 p-4"
            >
              <div className="flex items-start gap-3">
                <span
                  aria-hidden="true"
                  className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-emerald-900/60 bg-emerald-950/50 text-[10px] font-mono text-emerald-400"
                >
                  {index + 1}
                </span>

                <div className="min-w-0 flex-1">
                  <p className="text-sm text-slate-300">{fix.description}</p>

                  <div className="flex flex-wrap items-center gap-2 mt-2">
                    <span
                      className={`text-[10px] uppercase tracking-wider font-mono px-2 py-0.5 rounded border ${getDifficultyStyles(fix.difficulty)}`}
                    >
                      {fix.difficulty}
                    </span>
                    {fix.requires_upgrade && (
                      <span
                        title="Applying this fix requires a contract upgrade or protocol change"
                        className="text-[10px] uppercase tracking-wider font-mono px-2 py-0.5 rounded border border-amber-900/60 bg-amber-950/40 text-amber-400"
                      >
                        ↑ Requires upgrade
                      </span>
                    )}
                  </div>

                  {(fix.remedy_code || fix.example) && (
                    <div className="mt-3 space-y-2">
                      {fix.remedy_code && (
                        <CodeBlock label="Remedy code" code={fix.remedy_code} />
                      )}
                      {fix.example && (
                        <CodeBlock label="Example" code={fix.example} />
                      )}
                    </div>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
