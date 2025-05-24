// Ctrl+Enter (CSI‑u 13;5u) should submit the buffer.

import { renderTui } from "./ui-test-helpers.js";
import MultilineTextEditor from "../src/components/chat/multiline-editor.js";
import * as React from "react";
import { describe, it, expect, vi } from "vitest";

// Enable debug logging
process.env['TEXTBUFFER_DEBUG'] = '1';



describe("MultilineTextEditor – Ctrl+Enter submits", () => {
  it("calls onSubmit when CSI 13;5u is received", async () => {
    const onSubmit = vi.fn();

    const { stdin, flush, cleanup } = renderTui(
      React.createElement(MultilineTextEditor, {
        height: 5,
        width: 20,
        onSubmit,
      }),
    );

    // Initial render flush
    await flush();

    // Type "hello" followed immediately by Ctrl+Enter
    stdin.write("hello");
    stdin.write("[13;5u"); // Ctrl+Enter (modifier 5 = Ctrl)
    
    // Single flush after all input
    await flush();

    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(onSubmit.mock.calls[0]![0]).toBe("hello");

    cleanup();
  });
});
