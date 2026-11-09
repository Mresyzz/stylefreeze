use stylefreeze::check::run_check;
use stylefreeze::engine::SurgicalEngine;
use stylefreeze::lang::SupportedLang;
use std::path::Path;

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

    assert!(reconciled.contains("maxRetries: config.retries + 1,"));
    assert!(reconciled.contains("    client.close();"));
    assert!(reconciled.contains("    timeout: number;"));
    assert!(stats.discarded_churn_lines > 0);
    assert!(stats.estimated_tokens_saved > 0);
}

#[test]
fn test_check_on_clean_repo() {
    let repo_dir = Path::new(".");
    let (results, all_clean) = run_check(repo_dir, false, false).unwrap();
    // On clean working tree, should be clean
    if all_clean {
        assert_eq!(results.len(), 0);
    }
}
