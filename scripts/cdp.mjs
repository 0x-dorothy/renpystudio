// Minimal Chrome DevTools Protocol client for the Tauri WebView2 window.
// Usage:
//   node scripts/cdp.mjs shot out.png
//   node scripts/cdp.mjs eval "document.title"
//   node scripts/cdp.mjs click "text=Добавить блок"
//   node scripts/cdp.mjs wait 1500
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
  if (response.error) throw new Error(JSON.stringify(response.error));
  const result = response.result?.result;
  if (response.result?.exceptionDetails) {
    throw new Error(response.result.exceptionDetails.exception?.description ?? "eval failed");
  }
  return result?.value;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function clickExpression(target) {
  const selector = target.startsWith("text=")
    ? null
    : target;
  const text = target.startsWith("text=") ? target.slice(5) : null;
  return `(() => {
    const text = ${JSON.stringify(text)};
    const selector = ${JSON.stringify(selector)};
    const nodes = selector ? Array.from(document.querySelectorAll(selector)) : Array.from(document.querySelectorAll('button, [role="button"], .card, .tab'));
    const el = text ? nodes.find(n => (n.innerText || '').trim().includes(text)) : nodes[0];
    if (!el) return 'not found';
    el.scrollIntoView({block: 'center'});
    el.click();
    return (el.innerText || el.className).slice(0, 80);
  })()`;
}

switch (command) {
  case "shot": {
    const { result } = await send("Page.captureScreenshot", { format: "png" });
    writeFileSync(rest[0] ?? "shot.png", Buffer.from(result.data, "base64"));
    console.log("saved", rest[0] ?? "shot.png");
    break;
  }
  case "eval": {
    console.log(JSON.stringify(await evaluate(rest.join(" "))));
    break;
  }
  case "click": {
    console.log(await evaluate(clickExpression(rest.join(" "))));
    await sleep(500);
    break;
  }
  case "type": {
    const [selector, value] = [rest[0], rest.slice(1).join(" ")];
    console.log(
      await evaluate(`(() => {
        const el = document.querySelector(${JSON.stringify(selector)});
        if (!el) return 'not found';
        const setter = Object.getOwnPropertyDescriptor(el.constructor.prototype, 'value').set;
        setter.call(el, ${JSON.stringify(value)});
        el.dispatchEvent(new Event('input', { bubbles: true }));
        return 'ok';
      })()`),
    );
    await sleep(300);
    break;
  }
  case "wait": {
    await sleep(Number(rest[0] ?? 500));
    console.log("waited");
    break;
  }
  default:
    console.error("unknown command", command);
    process.exit(1);
}

ws.close();
