// Dev-only helper: drives the app's WebView2 through the Chrome DevTools Protocol.
// Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
// then: node scripts/cdp.mjs shot out.png | eval "document.title" | click "text=Граф"
import { writeFileSync } from "node:fs";

const [, , command, ...rest] = process.argv;

const targets = await (await fetch("http://127.0.0.1:9222/json/list")).json();
const page = targets.find((t) => t.type === "page" && !t.url.startsWith("devtools"));
if (!page) {
  console.error("no page target", targets.map((t) => t.url));
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});

let nextId = 1;
const pending = new Map();
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
  }
};

function send(method, params = {}) {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const response = await send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (response.result?.exceptionDetails) {
    throw new Error(response.result.exceptionDetails.exception?.description ?? "eval failed");
  }
  return response.result?.result?.value;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

switch (command) {
  case "shot": {
    const { result } = await send("Page.captureScreenshot", { format: "png" });
    writeFileSync(rest[0] ?? "shot.png", Buffer.from(result.data, "base64"));
    console.log("saved", rest[0] ?? "shot.png");
    break;
  }
  case "eval":
    console.log(JSON.stringify(await evaluate(rest.join(" "))));
    break;
  case "click": {
    const target = rest.join(" ");
    const selector = target.startsWith("text=") ? null : target;
    const text = target.startsWith("text=") ? target.slice(5) : null;
    console.log(
      await evaluate(`(() => {
        const text = ${JSON.stringify(text)};
        const selector = ${JSON.stringify(selector)};
        const nodes = selector ? Array.from(document.querySelectorAll(selector)) : Array.from(document.querySelectorAll('button, [role="button"], .card, .tab'));
        const el = text ? nodes.find(n => (n.innerText || '').trim().includes(text)) : nodes[0];
        if (!el) return 'not found';
        el.click();
        return (el.innerText || el.className).slice(0, 80);
      })()`),
    );
    await sleep(500);
    break;
  }
  case "wait":
    await sleep(Number(rest[0] ?? 500));
    console.log("waited");
    break;
  default:
    console.error("unknown command", command);
    process.exit(1);
}

ws.close();
