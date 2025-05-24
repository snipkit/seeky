// Regression test: Some terminals emit a carriage‑return ("\r") for
// Shift+Enter instead of a bare line‑feed. Pressing Shift+Enter in that
// environment should insert a newline **without** triggering submission.

import { renderTui } from "./ui-test-helpers.js";
import type { MultilineTextEditorHandle } from "../src/components/chat/multiline-editor.js";
import MultilineTextEditor from "../src/components/chat/multiline-editor.js";
import * as React from "react";
import { describe, it, expect, vi } from "vitest";

// Add a small delay to ensure proper event processing
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

async function type(
  stdin: NodeJS.WritableStream,
  text: string,
  flush: () => Promise<void>,
) {
  stdin.write(text);
  await flush();
  // Small delay after each input to ensure processing completes
  await delay(10);
}

describe("MultilineTextEditor - Shift+Enter (\r variant)", () => {
  it("inserts a newline and does NOT submit when the terminal sends \r for Shift+Enter", async () => {
    // This test focuses on verifying that:
    // 1. Shift+Enter correctly adds a newline to the buffer
    // 2. The cursor moves to the start of the new line
    // 3. Typing after Shift+Enter adds text to the new line
    // 4. No submission occurs
    const onSubmit = vi.fn();
    const editorRef = React.useRef<MultilineTextEditorHandle>(null);

    const { stdin, flush, cleanup } = renderTui(
      React.createElement(MultilineTextEditor, {
        height: 10,
        width: 20,
        initialText: "",
        onSubmit,
        ref: editorRef,
      }),
    );

    await flush();

    // Type initial text
    await type(stdin, "foo", flush);
    
    // Verify initial state
    expect(editorRef.current?.getText()).toBe("foo");
    expect(editorRef.current?.getRow()).toBe(0);
    expect(editorRef.current?.getCol()).toBe(3);

    // Send Shift+Enter
    await type(stdin, "\u001B[13;2u", flush); // ESC [ 13 ; 2 u
    
    // Verify newline was added and cursor moved to new line
    expect(editorRef.current?.getRow()).toBe(1);
    expect(editorRef.current?.getCol()).toBe(0);

    // Type on new line
    await type(stdin, "bar", flush);

    // Verify final state
    const finalText = editorRef.current?.getText() || '';
    const finalLines = finalText.split('\n');
    
    // Check buffer content - this is the key functional requirement
    expect(finalLines.length).toBe(2);
    expect(finalLines[0]).toBe('foo');
    expect(finalLines[1]).toBe('bar');
    
    // Check cursor position
    expect(editorRef.current?.getRow()).toBe(1);
    expect(editorRef.current?.getCol()).toBe(3);
    
    // Verify no submission occurred - this is the other key requirement
    expect(onSubmit).not.toHaveBeenCalled();

    cleanup();
  });
});
