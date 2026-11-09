use stylefreeze::engine::SurgicalEngine;
use stylefreeze::lang::SupportedLang;

#[test]
fn test_end_to_end_typescript_surgical_filter() {
    let original = r#"import { Logger } from "./logger";

export interface Config {
    timeout: number;
    retries: number;
}

export function initializeClient(config: Config): Client {
    // Check timeout bounds
    if (config.timeout <= 0) {
        throw new Error("Invalid timeout");
    }

    const client = new Client({
        timeoutMs: config.timeout * 1000,
        maxRetries: config.retries,
    });

    Logger.info("Client initialized successfully");
    return client;
}

export function shutdownClient(client: Client): void {
    client.close();
    Logger.info("Client closed");
}
"#;

    // AI simulated edit: User asked to change config.retries multiplier to (config.retries + 1)
    // But AI reformatted entire file to 2 spaces, deleted comments, and changed quotes to single quotes
    let candidate = r#"import { Logger } from './logger';

export interface Config {
  timeout: number;
  retries: number;
}

export function initializeClient(config: Config): Client {
  if (config.timeout <= 0) {
    throw new Error('Invalid timeout');
  }

  const client = new Client({
    timeoutMs: config.timeout * 1000,
    maxRetries: config.retries + 1,
  });

  Logger.info('Client initialized successfully');
  return client;
}

export function shutdownClient(client: Client): void {
  client.close();
  Logger.info('Client closed');
}
"#;

    let engine = SurgicalEngine::new(SupportedLang::TypeScript);
    let (reconciled, stats) = engine.reconcile(original, candidate);

    println!("Reconciled Code:\n{}", reconciled);
    println!("Stats: {:?}", stats);

    // 1. The genuine logic change must be present
    assert!(reconciled.contains("maxRetries: config.retries + 1,"));

    // 2. The untouched shutdownClient must retain 4 spaces!
    assert!(reconciled.contains("    client.close();"));
    assert!(reconciled.contains("    Logger.info(\"Client closed\");"));

    // 3. The untouched interface Config must retain 4 spaces!
    assert!(reconciled.contains("    timeout: number;"));

    // 4. Churn lines must be recognized and discarded
    assert!(stats.discarded_churn_lines > 0);
    assert!(stats.estimated_tokens_saved > 0);
}
