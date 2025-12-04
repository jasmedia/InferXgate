import http from "k6/http";
import { check, sleep } from "k6";
import { Trend, Counter, Rate } from "k6/metrics";
import { textSummary } from "https://jslib.k6.io/k6-summary/0.0.1/index.js";
import {
  CONFIG,
  getTargetUrl,
  getHeaders,
  getChatPayload,
  TEST_PROMPTS,
} from "./helpers/config.js";

// Custom metrics
const requestLatency = new Trend("chat_completion_latency", true);
const requestsTotal = new Counter("requests_total");
const errorRate = new Rate("error_rate");
const tokensPerSecond = new Trend("tokens_per_second");

export const options = {
  scenarios: {
    sustained_load: {
      executor: "constant-vus",
      vus: 10,
      duration: "10m",
    },
  },
  thresholds: {
    chat_completion_latency: ["p(95)<45000"],
    error_rate: ["rate<0.1"],
  },
};

export default function () {
  const url = `${getTargetUrl()}/v1/chat/completions`;
  const headers = getHeaders();
  const promptIndex = __ITER % TEST_PROMPTS.length;
  const payload = getChatPayload(TEST_PROMPTS[promptIndex]);

  const startTime = Date.now();
  const response = http.post(url, payload, {
    headers: headers,
    timeout: "120s",
    tags: { target: CONFIG.TARGET, test_type: "sustained" },
  });
  const duration = Date.now() - startTime;

  // Record metrics
  requestLatency.add(duration);
  requestsTotal.add(1);

  const success = check(response, {
    "status is 200": (r) => r.status === 200,
    "has valid response": (r) => {
      try {
        return (
          JSON.parse(r.body).choices && JSON.parse(r.body).choices.length > 0
        );
      } catch (e) {
        return false;
      }
    },
  });

  if (success) {
    errorRate.add(0);
    try {
      const body = JSON.parse(response.body);
      if (body.usage && body.usage.completion_tokens) {
        tokensPerSecond.add(body.usage.completion_tokens / (duration / 1000));
      }
    } catch (e) {
      // Ignore parsing errors
    }
  } else {
    errorRate.add(1);
    console.log(
      `Request failed: status=${response.status}, body=${response.body}`,
    );
  }

  sleep(1);
}

export function handleSummary(data) {
  const target = CONFIG.TARGET;
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  const resultsDir = __ENV.RESULTS_DIR || "results";

  // Custom summary output
  const metrics = data.metrics;
  let customOutput = "\n" + "=".repeat(60) + "\n";
  customOutput +=
    "SUSTAINED LOAD TEST RESULTS - " + target.toUpperCase() + "\n";
  customOutput += "=".repeat(60) + "\n\n";

  if (metrics.chat_completion_latency) {
    const lat = metrics.chat_completion_latency.values;
    customOutput += "Latency (ms):\n";
    customOutput += "  min:  " + lat.min.toFixed(2) + "\n";
    customOutput += "  avg:  " + lat.avg.toFixed(2) + "\n";
    customOutput += "  med:  " + lat.med.toFixed(2) + "\n";
    customOutput += "  p90:  " + lat["p(90)"].toFixed(2) + "\n";
    customOutput += "  p95:  " + lat["p(95)"].toFixed(2) + "\n";
    customOutput += "  p99:  " + lat["p(99)"].toFixed(2) + "\n";
    customOutput += "  max:  " + lat.max.toFixed(2) + "\n\n";
  }

  if (metrics.requests_total) {
    const duration = data.state.testRunDurationMs / 1000;
    const total = metrics.requests_total.values.count;
    customOutput += "Requests:\n";
    customOutput += "  total:        " + total + "\n";
    customOutput += "  requests/sec: " + (total / duration).toFixed(2) + "\n\n";
  }

  if (metrics.tokens_per_second) {
    const tps = metrics.tokens_per_second.values;
    customOutput += "Tokens/Second:\n";
    customOutput += "  avg: " + tps.avg.toFixed(2) + "\n";
    customOutput += "  max: " + tps.max.toFixed(2) + "\n\n";
  }

  if (metrics.error_rate) {
    customOutput +=
      "Error Rate: " +
      (metrics.error_rate.values.rate * 100).toFixed(2) +
      "%\n";
  }

  customOutput += "=".repeat(60) + "\n";

  const jsonFile =
    resultsDir + "/sustained-" + target + "-" + timestamp + ".json";

  return {
    [jsonFile]: JSON.stringify(data, null, 2),
    stdout:
      customOutput +
      "\n" +
      textSummary(data, { indent: " ", enableColors: true }),
  };
}
