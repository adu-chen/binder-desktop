//! Development-only fixed transport. Never uses the application's database or workspace.
use crate::{
    ag, ent,
    ipc::{AgTurnEvent, IpcError},
    AppState,
};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

static RUN: Mutex<Option<(String, Arc<AppState>, Arc<Mutex<String>>)>> = Mutex::new(None);
fn failure(message: impl ToString) -> IpcError {
    IpcError::new("AG_VERIFICATION_ERROR", &message.to_string(), false, false)
}

struct Sandbox(std::path::PathBuf);
impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("[ag-verification] temporary workspace cleanup failed: {error}");
        }
    }
}
struct Lease(String);
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut slot) = RUN.lock() {
            if slot.as_ref().is_some_and(|(id, _, _)| id == &self.0) {
                *slot = None;
            }
        }
    }
}

struct Gateway {
    url: String,
    request_paths: Arc<Mutex<Vec<String>>>,
    tool_messages: Arc<Mutex<Vec<serde_json::Value>>>,
    prompt_observations: Arc<Mutex<Vec<serde_json::Value>>>,
    stopped: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<(), String>>>,
}
impl Gateway {
    fn start(scenario: &str) -> Result<Self, IpcError> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(failure)?;
        listener.set_nonblocking(true).map_err(failure)?;
        let url = format!("http://{}", listener.local_addr().map_err(failure)?);
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let request_paths = Arc::new(Mutex::new(Vec::new()));
        let observed_paths = request_paths.clone();
        let tool_messages = Arc::new(Mutex::new(Vec::new()));
        let captured = tool_messages.clone();
        let prompt_observations = Arc::new(Mutex::new(Vec::new()));
        let observed_prompts = prompt_observations.clone();
        let scenario = scenario.to_string();
        let worker = std::thread::spawn(move || -> Result<(), String> {
            let mut replies = responses(&scenario);
            while !stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(socket) => socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => return Err(error.to_string()),
                };
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| e.to_string())?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                let path = loop {
                    let mut chunk = [0; 4096];
                    let n = stream.read(&mut chunk).map_err(|e| e.to_string())?;
                    if n == 0 {
                        return Err("request truncated".into());
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if bytes.len() > 1024 * 1024 {
                        return Err("request too large".into());
                    }
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..end]);
                        let length = header
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            if header.split_whitespace().nth(1) == Some("/invoke/stream") {
                                let request: serde_json::Value =
                                    serde_json::from_slice(&bytes[end + 4..end + 4 + length])
                                        .map_err(|e| e.to_string())?;
                                let request_messages = request["messages"]
                                    .as_array()
                                    .ok_or("missing model messages")?;
                                if scenario == "read-failure-clarification"
                                    || scenario == "repeated-oversized-read"
                                    || scenario == "repeated-read-tools-blocked"
                                    || scenario == "execution-budget-wrap"
                                {
                                    observed_prompts.lock().map_err(|error|error.to_string())?.push(serde_json::json!({
                                        "nativeToolCount": request["tools"].as_array().map_or(0,Vec::len),
                                    }));
                                }
                                if scenario == "session-memory" {
                                    let contains_in_layer = |layer: &str, marker: &str| {
                                        request_messages.iter().any(|message| {
                                            message["content"].as_str().is_some_and(|content| {
                                                content.contains(layer) && content.contains(marker)
                                            })
                                        })
                                    };
                                    observed_prompts.lock().map_err(|e| e.to_string())?.push(
                                        serde_json::json!({
                                            "oldHistory": contains_in_layer("[DATA-ONLY session_history]", "H07-OLD"),
                                            "correctedHistory": contains_in_layer("[DATA-ONLY session_history]", "H07-CORRECTED"),
                                            "longAnswer": contains_in_layer("[DATA-ONLY session_history]", "H07-LONG-HEAD") && contains_in_layer("[DATA-ONLY session_history]", "H07-LONG-TAIL"),
                                            "selectedMemory": contains_in_layer("[DATA-ONLY explicit_reference]", "H07-MEMORY"),
                                            "memoryOutsideReference": request_messages.iter().any(|message| message["content"].as_str().is_some_and(|content| content.contains("H07-MEMORY") && !content.contains("[DATA-ONLY explicit_reference]"))),
                                        }),
                                    );
                                }
                                if scenario == "bounded-reading-four-pages" {
                                    let history = request_messages
                                        .iter()
                                        .find_map(|message| {
                                            message["content"].as_str().filter(|text| {
                                                text.contains("[DATA-ONLY session_history]")
                                            })
                                        })
                                        .unwrap_or("");
                                    observed_prompts.lock().map_err(|e|e.to_string())?.push(serde_json::json!({
                                        "historyUserVisible": history.contains("CAPACITY-USER：只读，不修改"),
                                        "historyHeadVisible": history.contains("CAPACITY-HEAD"),
                                        "historyTailVisible": history.contains("CAPACITY-TAIL"),
                                        "historyExcerpted": history.contains("非完整消息"),
                                    }));
                                }
                                let mut messages = captured.lock().map_err(|e| e.to_string())?;
                                // Only this isolated fixture's actual tool reinjection, never tokens or prompts.
                                *messages = request["messages"]
                                    .as_array()
                                    .ok_or("missing model messages")?
                                    .iter()
                                    .filter(|message| message["role"] == "tool")
                                    .map(|message| {
                                        serde_json::json!({
                                            "toolCallId": message["tool_call_id"],
                                            "content": message["content"],
                                        })
                                    })
                                    .collect();
                            }
                            break header.split_whitespace().nth(1).unwrap_or("").to_string();
                        }
                    }
                };
                observed_paths
                    .lock()
                    .map_err(|error| error.to_string())?
                    .push(path.clone());
                let (status, mime, body) = match path.as_str() {
                    "/auth/login" => ("200 OK", "application/json", r#"{"token":"development-fixture"}"#.into()),
                    "/gate/check" => ("200 OK", "application/json", serde_json::json!({"verdict": if scenario == "gate-denied" || (scenario == "text-correction-gate-denied" && observed_paths.lock().map_err(|e|e.to_string())?.iter().filter(|path|path.as_str()=="/gate/check").count()>1) || (scenario == "read-failure-gate-denied" && replies.len()==1) { "denied" } else { "passed" }}).to_string()),
                    "/invoke/stream" => {
                        if scenario == "delay" || scenario == "budget-recovery-delay"
                            || (scenario == "text-correction-delay" && replies.len()==1) {
                            let until = Instant::now() + Duration::from_secs(20);
                            while Instant::now() < until && !stop.load(Ordering::Acquire) { std::thread::sleep(Duration::from_millis(10)); }
                            if stop.load(Ordering::Acquire) { break; }
                        }
                        match replies.pop_front() {
                            Some(body) => {
                                // A scripted provider consumes the real first-page receipt. No
                                // hardcoded version or replacement of the production executor.
                                let body = if body.contains("VERIFICATION_PAGE_HASH") {
                                    let captured = captured.lock().map_err(|e|e.to_string())?;
                                    let hash = captured.iter().rev().find_map(|message| {
                                        let value:serde_json::Value=serde_json::from_str(message["content"].as_str()?).ok()?;
                                        value["toolResult"]["version_hash"].as_str().map(str::to_owned)
                                    }).ok_or("missing real page version")?;
                                    body.replace("VERIFICATION_PAGE_HASH", &hash)
                                } else { body };
                                let body = if body.contains("VERIFICATION_TEXT_OFFSET") {
                                    let captured = captured.lock().map_err(|error|error.to_string())?;
                                    let offset = captured.iter().rev().find_map(|message| {
                                        let value:serde_json::Value=serde_json::from_str(message["content"].as_str()?).ok()?;
                                        value["toolResult"]["next_offset"].as_u64()
                                    }).ok_or("missing real text cursor")?;
                                    body.replace("\\\"VERIFICATION_TEXT_OFFSET\\\"", &offset.to_string())
                                } else { body };
                                let body = if body.contains("VERIFICATION_NEXT_CURSOR") {
                                    let captured = captured.lock().map_err(|error|error.to_string())?;
                                    let cursor = captured.iter().rev().find_map(|message| {
                                        let value:serde_json::Value=serde_json::from_str(message["content"].as_str()?).ok()?;
                                        value["toolResult"]["next_cursor"].as_str().map(str::to_owned)
                                    }).ok_or("missing real next cursor")?;
                                    body.replace("VERIFICATION_NEXT_CURSOR", &cursor)
                                } else { body };
                                ("200 OK", "text/event-stream", body)
                            },
                            None => ("500 Internal Server Error", "text/plain", "fixed responses exhausted".into()),
                        }
                    }
                    _ => ("404 Not Found", "text/plain", "unsupported fixture endpoint".into()),
                };
                if let Err(error) = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()) {
                    if !stop.load(Ordering::Acquire) { return Err(error.to_string()); }
                }
            }
            Ok(())
        });
        Ok(Self {
            url,
            request_paths,
            tool_messages,
            prompt_observations,
            stopped,
            worker: Some(worker),
        })
    }
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => (),
                other => eprintln!("[ag-verification] transport stopped with error: {other:?}"),
            }
        }
    }
}

fn control(action: &str) -> serde_json::Value {
    serde_json::json!({"version":"binder-turn-control-v1","nextAction":action,
        "intent":{"goal":"验证运行路径","intentType":"inspect","targetCandidates":[],"sideEffectIntent":"none","ambiguity":"none","requiredContext":[],"confirmationPoints":[]},
        "route":{"summary":"开发隔离验证","steps":[]}})
}
fn decision_response(summary: &str) -> String {
    let arguments = serde_json::json!({
        "kind": "manual_confirm",
        "summary": summary,
        "detail": "隔离验证：仅记录裁决，不执行文档或工作区修改",
        "_binderTurn": control("tool"),
    });
    format!(
        "event: tool_calls\ndata: {}\n\n",
        serde_json::json!({"tool_calls":[{"id":format!("verification-{summary}"),"name":"propose_decision","arguments":arguments.to_string()}]})
    )
}
fn responses(scenario: &str) -> VecDeque<String> {
    let text = format!("<binder-turn-control>{}</binder-turn-control>固定响应片段结束；请查看实际工具证据，不代表业务成功。", control("final"));
    let final_response = format!(
        "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
        serde_json::json!({"text":text})
    );
    let text_response = |text: &str| {
        format!(
            "event: chunk\ndata: {}\n\nevent: done\ndata: {{\"stop_reason\":\"stop\"}}\n\n",
            serde_json::json!({"text":text})
        )
    };
    let missing = text_response("PRIVATE_UNVALIDATED_TEXT");
    let glob = || {
        format!(
            "event: tool_calls\ndata: {}\n\n",
            serde_json::json!({"tool_calls":[{
                "id":"text-protocol-glob", "name":"workspace_glob", "arguments":serde_json::json!({"pattern":"*.txt","_binderTurn":control("tool")}).to_string()
            }]})
        )
    };
    match scenario {
        "native-business-tools" => {
            let native=|id:&str,args:serde_json::Value| format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":id,"name":"list_docs","arguments":args.to_string()}]}));
            VecDeque::from([
                native("verification-native-first",serde_json::json!({"_binderTurn":control("tool")})),
                native("verification-native-missing",serde_json::json!({})),
                native("verification-native-type",serde_json::json!({"_binderTurn":"PRIVATE_BAD_CALL"})),
                format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[
                    {"id":"verification-native-list","name":"list_docs","arguments":serde_json::json!({"_binderTurn":control("tool")}).to_string()},
                    {"id":"verification-native-glob","name":"workspace_glob","arguments":serde_json::json!({"pattern":"*.txt"}).to_string()}
                ]})),
                final_response,
            ])
        },
        "tool-description-compatible" => {
            let calls=[("list_docs",serde_json::json!({})),("workspace_glob",serde_json::json!({"pattern":"*.txt"})),("workspace_glob",serde_json::json!({"pattern":"*.md"}))].into_iter().enumerate().map(|(index,(name,mut args))| {
                let mut semantic=control("tool"); semantic["intent"]["goal"]=serde_json::json!(format!("private goal {index}"));
                semantic["route"]["summary"]=serde_json::json!(format!("private route {index}")); semantic["route"]["steps"]=serde_json::json!([format!("private step {index}")]);
                args["_binderTurn"]=semantic; serde_json::json!({"id":format!("description-{index}"),"name":name,"arguments":args.to_string()})
            }).collect::<Vec<_>>();
            VecDeque::from([format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":calls})),final_response])
        }
        "tool-read-only-control-variants" => {
            let primary = control("tool");
            let mut duplicate = primary.clone();
            duplicate["intent"]["requiredContext"] = serde_json::json!(["PRIVATE_BLOCK_HINT"]);
            let calls = serde_json::json!({"tool_calls":[
                {"id":"read-primary","name":"list_docs","arguments":serde_json::json!({"_binderTurn":primary}).to_string()},
                {"id":"read-duplicate","name":"workspace_glob","arguments":serde_json::json!({"pattern":"*.txt","_binderTurn":duplicate}).to_string()}
            ]});
            VecDeque::from([format!("event: tool_calls\ndata: {calls}\n\n"), final_response])
        }
        "protocol-cumulative" => {
            let call=|id:&str,semantic:serde_json::Value| format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":id,"name":"list_docs","arguments":serde_json::json!({"_binderTurn":semantic}).to_string()}]}));
            let mut bad=control("tool"); bad["intent"]["ambiguity"]=serde_json::json!("blocking");
            VecDeque::from([call("bad-first",bad),glob(),call("bad-second",control("final")),call("bad-third",control("final"))])
        }
        "action-handoff-plan" => {
            let mut pending=control("final"); pending["intent"]["sideEffectIntent"]=serde_json::json!("structure_plan");
            let invalid=text_response(&format!("<binder-turn-control>{pending}</binder-turn-control>UNDELIVERED_PLAN_PROMISE"));
            let final_text=text_response(&format!("<binder-turn-control>{pending}</binder-turn-control>仅依据实际计划与执行回执说明结果，不再创建重复计划。"));
            pending["nextAction"]=serde_json::json!("tool");
            let args=serde_json::json!({"operations":[{"kind":"rename_resource","sourcePath":"verification.txt","targetPath":"renamed.txt","resourceKind":"file"}],"_binderTurn":pending});
            VecDeque::from([invalid,format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":"handoff-resource-plan","name":"plan_workspace_operation","arguments":args.to_string()}]})),final_text])
        }
        "execution-budget-wrap" => VecDeque::from([glob(), final_response]),
        "action-handoff-stage" => {
            let mut pending=control("final"); pending["intent"]["sideEffectIntent"]=serde_json::json!("structure_plan");
            let invalid=text_response(&format!("<binder-turn-control>{pending}</binder-turn-control>UNDELIVERED_PLAN_PROMISE"));
            pending["completion"]=serde_json::json!({"status":"report_only","reason":"本次仅分析新增文件清单，未请求执行"});
            VecDeque::from([invalid,text_response(&format!("<binder-turn-control>{pending}</binder-turn-control>本次仅报告候选清单，未创建文件、未进入确认等待。"))])
        }
        "bounded-reading" | "bounded-reading-four-pages" => {
            let call = |id:&str, mut args:serde_json::Value| {
                args["_binderTurn"] = control("tool");
                format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":id,"name":"read_doc","arguments":args.to_string()}]}))
            };
            let page_count = if scenario == "bounded-reading-four-pages" { 4 } else { 2 };
            let mut replies = vec![call("reading-first",serde_json::json!({"path":"reading.html"}))];
            for index in 1..page_count {
                replies.push(call(&format!("reading-next-{index}"),serde_json::json!({"path":"reading.html","cursor":"VERIFICATION_NEXT_CURSOR"})));
            }
            replies.push(final_response);
            replies.into()
        }
        "page-contract-recovery" | "page-contract-exhausted" => {
            let call = |id:&str, name:&str, mut args:serde_json::Value| {
                args["_binderTurn"]=control("tool");
                format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":id,"name":name,"arguments":args.to_string()}]}))
            };
            let read = call("page-read","read_doc",serde_json::json!({"path":"pages.txt","blocks":["0"]}));
            if scenario=="page-contract-exhausted" {
                std::iter::once(read).chain((0..3).map(|index|call(&format!("page-invalid-{index}"),"list_blocks",serde_json::json!({"path":"pages.txt","cursor":"bad-cursor","limit":20})))).collect()
            } else {
                vec![read,
                    call("page-invalid","list_blocks",serde_json::json!({"path":"pages.txt","cursor":"bad-cursor","limit":20})),
                    call("page-first","list_blocks",serde_json::json!({"path":"pages.txt","limit":20})),
                    call("page-next","list_blocks",serde_json::json!({"path":"pages.txt","cursor":"VERIFICATION_NEXT_CURSOR"})),final_response].into()
            }
        }
        "argument-decision-shared-limit" => {
            let args=serde_json::json!({"pattern":7,"_binderTurn":control("tool")});
            let invalid=format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":"bad-param-before-wait","name":"workspace_glob","arguments":args.to_string()}]}));
            vec![invalid,decision_response("arg-A"),missing.clone(),missing.clone(),missing].into()
        }
        "parameter-and-control-recover" => {
            let bad = |id: &str| format!(
                "event: tool_calls\ndata: {}\n\n",
                serde_json::json!({"tool_calls":[{"id":id,"name":"workspace_glob","arguments":serde_json::json!({"pattern":7,"_binderTurn":control("tool")}).to_string()}]})
            );
            vec![bad("bad-param-1"), bad("bad-param-2"), missing, glob(), final_response].into()
        }
        "decision-native-replay-blocked" => {
            let invalid=format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{"id":"missing-control-after-wait","name":"workspace_glob","arguments":"{\"pattern\":\"*.txt\"}"}]}));
            vec![decision_response("native-A"),invalid,decision_response("native-B"),decision_response("native-C")].into()
        }
        "text-control-corrects" => vec![missing, final_response].into(),
        "text-correction-gate-denied" | "text-correction-delay" => vec![missing,final_response].into(),
        "text-correction-incomplete" => vec![missing,final_response.split("event: done").next().unwrap().to_string()].into(),
        "text-correction-http402" => vec![missing,"event: error\ndata: {\"code\":\"HTTP_402\",\"message\":\"fixture balance rejected\",\"retriable\":false}\n\n".into()].into(),
        "text-invalid-corrects" => vec![text_response("<binder-turn-control>{bad}</binder-turn-control>PRIVATE_UNVALIDATED_TEXT"),final_response].into(),
        "text-incomplete-corrects" => vec![text_response("<binder-turn-control>{"),final_response].into(),
        "text-empty-corrects" => vec![text_response(&format!("<binder-turn-control>{}</binder-turn-control>",control("final"))),final_response].into(),
        "text-protocol-exhausted" => vec![missing;3].into(),
        "tool-preamble" => vec![format!("event: chunk\ndata: {{\"text\":\"PRIVATE_UNVALIDATED_TEXT\"}}\n\n{}",glob()),final_response].into(),
        "tool-text-invalid-corrects" => vec![format!("event: chunk\ndata: {{\"text\":\"<binder-turn-control>{{bad}}</binder-turn-control>\"}}\n\n{}",glob()),glob(),final_response].into(),
        "after-read-text-corrects" => vec![glob(),missing,final_response].into(),
        "decision-text-corrects" => vec![decision_response("text-A"),missing,final_response].into(),
        "decision-text-replay-blocked" => vec![decision_response("text-A"),missing,decision_response("text-B"),decision_response("text-C")].into(),
        "text-decision-shared-limit" => vec![missing.clone(),decision_response("text-A"),missing.clone(),missing].into(),
        "text-decision-model-budget" => std::iter::once(missing).chain((0..5).map(|i|decision_response(&format!("text-limit-{i}")))).collect(),
        "invalid-json" => vec!["event: tool_calls\ndata: {invalid\n\n".into()].into(),
        "truncated" => vec![format!(
            "event: chunk\ndata: {}\n\n",
            serde_json::json!({"text":format!("<binder-turn-control>{}</binder-turn-control>unfinished", control("final"))})
        )].into(),
        "glob" | "tool-failure" | "budget-after-tool" => {
            let (name, mut args) = match scenario {
                "glob" => ("workspace_glob", serde_json::json!({"pattern":"*.txt"})),
                "budget-after-tool" => ("workspace_glob", serde_json::json!({"pattern":"VERIFICATION_OVERSIZED_PATTERN".repeat(16000)})),
                _ => ("read_doc", serde_json::json!({"path":"missing.txt"})),
            };
            args["_binderTurn"] = control("tool");
            vec![format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"verification-call","name":name,"arguments":args.to_string()}]})), final_response].into()
        }
        "protocol-corrects" => {
            let invalid = serde_json::json!({"pattern":"*.txt"});
            let mut corrected = serde_json::json!({"pattern":"*.txt"});
            corrected["_binderTurn"] = control("tool");
            vec![
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"protocol-invalid","name":"workspace_glob","arguments":invalid.to_string()}]})),
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"protocol-corrected","name":"workspace_glob","arguments":corrected.to_string()}]})),
                final_response,
            ].into()
        }
        "argument-corrects" => {
            let mut invalid = serde_json::json!({"pattern":7});
            invalid["_binderTurn"] = control("tool");
            let mut corrected = serde_json::json!({"pattern":"*.txt"});
            corrected["_binderTurn"] = control("tool");
            vec![
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"argument-invalid","name":"workspace_glob","arguments":invalid.to_string()}]})),
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"argument-corrected","name":"workspace_glob","arguments":corrected.to_string()}]})),
                final_response,
            ].into()
        }
        "protocol-exhausted" => (0..3).map(|index| {
            format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":format!("protocol-invalid-{index}"),"name":"workspace_glob","arguments":serde_json::json!({"pattern":"*.txt"}).to_string()}]}))
        }).collect(),
        "read-failure-same-batch" => {
            vec![format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[
                {"id":"batch-read-1","name":"read_doc","arguments":serde_json::json!({"path":"oversized.txt","view":"structure","_binderTurn":control("tool")}).to_string()},
                {"id":"batch-read-2","name":"read_doc","arguments":"{\"path\":\"oversized.txt\",\"view\":\"structure\"}"},
                {"id":"batch-read-3","name":"workspace_glob","arguments":"{\"pattern\":\"*.txt\"}"}
            ]})),final_response].into()
        }
        "repeated-oversized-read" | "repeated-read-tools-blocked" | "read-failure-gate-denied" | "read-failure-with-evidence" | "read-failure-clarification" => {
            let mut args = serde_json::json!({"path":"oversized.txt","view":"structure"});
            args["_binderTurn"] = control("tool");
            let mut replies:VecDeque<_>=(0..2).map(|index| format!(
                "event: tool_calls\ndata: {}\n\n",
                serde_json::json!({"tool_calls":[{"id":format!("oversized-{index}"),"name":"read_doc","arguments":args.to_string()}]})
            )).collect();
            if scenario=="read-failure-with-evidence" {
                replies.push_front(format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{
                    "id":"prior-successful-read","name":"read_doc","arguments":serde_json::json!({"path":"verification.txt","_binderTurn":control("tool")}).to_string()
                }]})));
            }
            if scenario=="read-failure-clarification" {
                let mut clarify=control("clarify");
                clarify["intent"]["ambiguity"]="blocking".into();
                clarify["clarificationQuestion"]="请指定较小读取范围".into();
                replies.push_back(text_response(&format!("<binder-turn-control>{clarify}</binder-turn-control>请指定较小读取范围")));
                replies.push_back(format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{
                    "id":"clarified-narrow-read","name":"read_doc","arguments":serde_json::json!({"path":"oversized.txt","blocks":["0"],"_binderTurn":control("tool")}).to_string()
                }]})));
            }
            if scenario=="repeated-read-tools-blocked" {
                // The live synthetic probe selected edit_blocked despite final-only
                // instructions. Replay that response shape through the actual guard.
                replies.push_back(format!("event: tool_calls\ndata: {}\n\n",serde_json::json!({"tool_calls":[{
                    "id":"forbidden-block-report", "name":"edit_blocked",
                    "arguments":serde_json::json!({"code":"READ_BLOCKED","reason":"读取重复失败","_binderTurn":control("tool")}).to_string()
                }]})));
            }
            replies.push_back(final_response);
            replies
        }
        "oversized-read-recovery" => {
            let mut full = serde_json::json!({"path":"oversized.txt","view":"structure"});
            full["_binderTurn"] = control("tool");
            let mut index = serde_json::json!({"path":"oversized.txt"});
            index["_binderTurn"] = control("tool");
            let mut selected = serde_json::json!({"path":"oversized.txt","blocks":["0"]});
            selected["_binderTurn"] = control("tool");
            vec![
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"oversized-full","name":"read_doc","arguments":full.to_string()}]})),
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"oversized-index","name":"list_blocks","arguments":index.to_string()}]})),
                format!("event: tool_calls\ndata: {}\n\n", serde_json::json!({"tool_calls":[{"id":"oversized-selected","name":"read_doc","arguments":selected.to_string()}]})),
                final_response,
            ].into()
        }
        "decision-chain" => vec![
            decision_response("A"),
            decision_response("B"),
            final_response,
        ]
        .into(),
        "decision-duplicate" => vec![decision_response("A"), decision_response("A")].into(),
        "decision-limit" => (0..6).map(|index| decision_response(&format!("limit-{index}"))).collect(),
        "session-memory" => vec![
            final_response.clone(),
            final_response.clone(),
            final_response.clone(),
            final_response.clone(),
            decision_response("H07-A"),
            final_response,
        ]
        .into(),
        _ => vec![final_response].into(),
    }
}

// Runs the production turn entry with distinct AgentSession identities and an explicit
// MemorySelectionRef. The fixture observes only marker presence, never exports a prompt.
async fn verify_session_memory<R: tauri::Runtime>(
    gateway: &Gateway,
    state: &AppState,
    workspace_id: &str,
    run_id: &str,
    active_turn_id: &Arc<Mutex<String>>,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    app: tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    let session_b = uuid::Uuid::new_v4().to_string();
    let turn_ids: Vec<String> = std::iter::once(run_id.to_string())
        .chain((0..5).map(|_| uuid::Uuid::new_v4().to_string()))
        .collect();
    let prior = |turn_id: &str, content: &str| ag::context::HistTurn {
        workspace_id: workspace_id.into(),
        session_id: run_id.into(),
        role: "user".into(),
        content: content.into(),
        rel_path: None,
        turn_id: Some(turn_id.into()),
    };
    let old = prior(&turn_ids[0], "H07-OLD：项目代号为旧松");
    let corrected = prior(&turn_ids[1], "H07-CORRECTED：项目代号改为青松");
    let mut long_answer = old.clone();
    long_answer.role = "assistant".into();
    long_answer.content = format!(
        "H07-LONG-HEAD{}H07-LONG-TAIL",
        "历史回答：只读，不修改。".repeat(1000)
    );
    let prompts = [
        "H07-OLD：项目代号为旧松",
        "H07-CORRECTED：项目代号改为青松",
        "查看显式选择的项目记忆",
        "本轮未选择项目记忆，请继续",
        "创建等待用户裁决的隔离任务",
        "B 会话独立问候",
    ];
    for index in 0..turn_ids.len() {
        let session_id = if index == 5 { &session_b } else { run_id };
        let turn_id = &turn_ids[index];
        *active_turn_id.lock().map_err(failure)? = turn_id.clone();
        let facts = serde_json::json!({"version":"agent-context-facts-v2","workspaceId":workspace_id,"sessionId":session_id,"turnId":turn_id,"contextRevision":1,"activeTarget":null,"openResources":[],"selections":[],"proposalFacts":[]});
        let history = match index {
            0 | 5 => vec![],
            1 => vec![old.clone(), long_answer.clone()],
            _ => vec![old.clone(), long_answer.clone(), corrected.clone()],
        };
        let references = if index == 2 {
            vec![crate::types::ReferencePayload::Memory {
                entry_id: Some("h07-memory".into()),
                title: Some("项目代号".into()),
                content: "H07-MEMORY：项目代号为青松".into(),
                label: None,
            }]
        } else {
            vec![]
        };
        if index == 5 {
            let rejected_turn_id = uuid::Uuid::new_v4().to_string();
            let mut rejected_facts = facts.clone();
            rejected_facts["turnId"] = serde_json::json!(rejected_turn_id);
            let wrong_scope = ag::tool_registry::with_verification_execution_policy(
                ag::tool_registry::ExecutionBudgetPolicy::legacy(),
                ent::gateway::with_test_gateway(
                    gateway.url.clone(),
                    ag::turn_runtime::run_agent_turn(
                        rejected_turn_id,
                        session_b.clone(),
                        Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                        "跨会话历史应拒绝".into(),
                        Some("fixture".into()),
                        Some("fixture-model".into()),
                        None,
                        None,
                        None,
                        vec![],
                        Some(rejected_facts),
                        vec![old.clone()],
                        vec![],
                        vec![],
                        vec![],
                        None,
                        None,
                        None,
                        None,
                        None,
                        on_event.clone(),
                        ag::turn_runtime::AgentContextChannel(None),
                        state,
                        app.clone(),
                    ),
                ),
            )
            .await;
            if !matches!(wrong_scope, Err(ref error) if error.code == "AG_HISTORY_SCOPE_MISMATCH") {
                return Err(failure(
                    "Cross-session history was not rejected before Gate",
                ));
            }
        }
        ag::tool_registry::with_verification_execution_policy(
            ag::tool_registry::ExecutionBudgetPolicy::legacy(),
            ent::gateway::with_test_gateway(
                gateway.url.clone(),
                ag::turn_runtime::run_agent_turn(
                    turn_id.clone(),
                    session_id.to_string(),
                    Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                    prompts[index].into(),
                    Some("fixture".into()),
                    Some("fixture-model".into()),
                    None,
                    None,
                    None,
                    vec![],
                    Some(facts),
                    history,
                    vec![],
                    vec![],
                    references,
                    None,
                    None,
                    None,
                    None,
                    None,
                    on_event.clone(),
                    ag::turn_runtime::AgentContextChannel(None),
                    state,
                    app.clone(),
                ),
            ),
        )
        .await?;
    }
    let observations = gateway.prompt_observations.lock().map_err(failure)?.clone();
    let observed: Vec<_> = observations
        .iter()
        .map(|entry| {
            (
                entry["oldHistory"] == true,
                entry["correctedHistory"] == true,
                entry["selectedMemory"] == true,
                entry["memoryOutsideReference"] == true,
            )
        })
        .collect();
    if observed
        != [
            (false, false, false, false),
            (true, false, false, false),
            (true, true, true, false),
            (true, true, false, false),
            (true, true, false, false),
            (false, false, false, false),
        ]
    {
        return Err(failure(
            "Session history or explicit memory leaked across model requests",
        ));
    }
    if observations
        .iter()
        .enumerate()
        .any(|(index, entry)| entry["longAnswer"] != (index > 0 && index < 5))
    {
        return Err(failure(
            "Long assistant history displaced the prior request or leaked into another session",
        ));
    }
    let request_paths = gateway.request_paths.lock().map_err(failure)?;
    let gate_count = request_paths
        .iter()
        .filter(|path| path.as_str() == "/gate/check")
        .count();
    let model_count = request_paths
        .iter()
        .filter(|path| path.as_str() == "/invoke/stream")
        .count();
    if (gate_count, model_count) != (6, 6) {
        return Err(failure(
            "Cross-session rejection or waiting turn changed gateway call count",
        ));
    }
    let conn = state.db.lock().map_err(failure)?;
    let mut events = Vec::new();
    let mut statuses = Vec::new();
    for turn_id in &turn_ids {
        let (status, persisted_session): (String, String) = conn
            .query_row(
                "SELECT status, session_id FROM agent_turns WHERE turn_id=?1",
                [turn_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(failure)?;
        statuses.push(
            serde_json::json!({"turnId":turn_id,"sessionId":persisted_session,"status":status}),
        );
        events.extend(
            ag::turn_events::list_safe_events(&conn, workspace_id, turn_id, 0).map_err(failure)?,
        );
    }
    let waiting: (String, String) = conn.query_row(
        "SELECT d.request_id, r.status FROM decision_requests d JOIN task_runs r ON r.task_run_id=d.task_run_id WHERE r.workspace_id=?1 AND r.session_id=?2 AND d.status='proposed'",
        rusqlite::params![workspace_id, run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(failure)?;
    let memory_entry_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM memory_entries", [], |row| row.get(0))
        .map_err(failure)?;
    if waiting.1 != "waiting"
        || statuses[5]["status"] != "completed"
        || statuses[5]["sessionId"] != session_b
        || statuses[..4]
            .iter()
            .any(|turn| turn["status"] != "completed")
        || events
            .iter()
            .filter(|event| event.event.terminal_status().is_some())
            .count()
            != 5
        || memory_entry_count != 0
    {
        return Err(failure(
            "Session isolation or pending DecisionRequest state did not persist",
        ));
    }
    Ok(serde_json::json!({
        "runId":run_id,"status":"session_memory_checked","events":events,
        "budgetChecks":[],"requestCounts":{"gate":gate_count,"model":model_count},
        "decisionTrace":[],"modelToolMessages":[],"promptTrace":observations,
        "sessionTrace":{"turns":statuses,"waitingRequestId":waiting.0,"waitingTaskStatus":waiting.1,"crossSessionHistoryRejectedBeforeGate":true,"memoryEntryCount":memory_entry_count},
        "environment":"development-isolated","realSubscriptionVerified":false,
    }))
}

// Isolated contract probe: uses the production checkpoint/transaction guards, not an editor.
fn verify_preview_guard(
    state: &AppState,
    workspace_id: &str,
    session_id: &str,
) -> Result<serde_json::Value, IpcError> {
    let conn = state.db.lock().map_err(failure)?;
    let task_run_id =
        ag::task::begin_turn(&conn, workspace_id, session_id, Some("预览检查点隔离验证"))
            .map_err(failure)?;
    let diff_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO pending_diffs (diff_id, workspace_id, rel_path, origin_module, status, payload_ref, created_at) VALUES (?1, ?2, 'verification.txt', 'AG', 'pending', 'isolated-preview', 1)",
        rusqlite::params![&diff_id, workspace_id],
    ).map_err(failure)?;
    ag::task::create_preview_checkpoint(
        &conn,
        &task_run_id,
        workspace_id,
        session_id,
        session_id,
        &diff_id,
        "verification.txt",
    )
    .map_err(failure)?;
    let v1 = ag::task::PreviewCheckpoint {
        diff_id: diff_id.clone(),
        task_run_id: task_run_id.clone(),
        workspace_id: workspace_id.into(),
        session_id: session_id.into(),
        turn_id: session_id.into(),
        rel_path: "verification.txt".into(),
        session_version: 1,
        content_hash: "verification-v1".into(),
    };
    let mut wrong_identity = v1.clone();
    wrong_identity.session_id = "unrelated-session".into();
    let rejected_identity = ag::turn_control::commit_preview_ack(&conn, &wrong_identity).is_err();
    let still_awaiting = ag::task::acknowledged_preview_checkpoint(
        &conn,
        &task_run_id,
        workspace_id,
        session_id,
        &diff_id,
    )
    .map_err(failure)?
    .is_none();
    ag::turn_control::commit_preview_ack(&conn, &v1)?;
    let duplicate_ack_rejected = ag::turn_control::commit_preview_ack(&conn, &v1).is_err();
    let mut v2 = v1.clone();
    v2.session_version = 2;
    v2.content_hash = "verification-v2".into();
    let stale_continuation_rejected = ag::task::begin_preview_continuation(&conn, &v2)
        .map_err(failure)?
        .is_none();
    let valid_continuation_once = ag::task::begin_preview_continuation(&conn, &v1)
        .map_err(failure)?
        == Some(task_run_id.clone());
    let duplicate_continuation_rejected = ag::task::begin_preview_continuation(&conn, &v1)
        .map_err(failure)?
        .is_none();
    if !(rejected_identity
        && still_awaiting
        && duplicate_ack_rejected
        && stale_continuation_rejected
        && valid_continuation_once
        && duplicate_continuation_rejected)
    {
        return Err(failure("Preview checkpoint guard failed"));
    }
    Ok(serde_json::json!({
        "taskRunId": task_run_id, "diffId": diff_id,
        "wrongIdentityRejectedBeforeAck": rejected_identity,
        "checkpointStillAwaitingAfterRejection": still_awaiting,
        "duplicateAckRejected": duplicate_ack_rejected,
        "staleV2ContinuationRejected": stale_continuation_rejected,
        "matchingV1ContinuationOnce": valid_continuation_once,
        "duplicateContinuationRejected": duplicate_continuation_rejected,
        "scope": "isolated_checkpoint_contract_only_no_editor_no_timeout_no_model",
    }))
}

async fn verify_effect_port<R: tauri::Runtime>(
    state: &AppState,
    _workspace: &str,
    root: &std::path::Path,
    run: &str,
    app: &tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    let mut traces = Vec::new();
    for (cancel, resource_plan) in [(true, false), (false, false), (true, true), (false, true)] {
        // Independent cases must not erase or bypass an earlier unknown-effect guard.
        // Each case owns its own migrated database and active-turn registry.
        let mut conn = rusqlite::Connection::open_in_memory().map_err(failure)?;
        crate::db::migrate(&mut conn).map_err(failure)?;
        let ws = crate::workspace::lifecycle::open_workspace(
            &mut conn,
            root.to_str()
                .ok_or_else(|| failure("Invalid temporary path"))?,
        )
        .map_err(failure)?;
        let state = &AppState {
            db: Arc::new(Mutex::new(conn)),
            registry: state.registry.clone(),
            assist_terminals: Arc::new(Mutex::new(HashMap::new())),
            active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
        };
        let workspace = ws.workspace_id.as_str();
        let turn = format!("{run}-effect-{cancel}-{resource_plan}");
        let session = turn.clone();
        {
            let conn = state.db.lock().map_err(failure)?;
            ag::history::ensure_session(
                &conn,
                workspace,
                &session,
                "isolated effect",
                "edit",
                0,
                true,
            )
            .map_err(failure)?;
            ag::history::save_turn(
                &conn,
                workspace,
                &session,
                &turn,
                None,
                "draft",
                &ag::history::AgentTurnPayload {
                    version: 1,
                    last_event_seq: 0,
                    user_message: ag::history::PersistedChatMessage {
                        id: format!("u-{turn}"),
                        role: "user".into(),
                        content: "isolated effect contract".into(),
                        parts: None,
                        streaming: None,
                        error: None,
                        segments: None,
                    },
                    assistant_message: None,
                    tool_summaries: vec![],
                    diff_refs: vec![],
                    failure: None,
                    incomplete_reason: None,
                },
            )
            .map_err(failure)?;
        }
        let (lease, cancelled, _, _, _) = ag::turn_control::register_active_agent_turn(
            state,
            workspace,
            &session,
            &turn,
            ag::turn_control::AgentTurnResumeKind::New,
        )?;
        let mut targets = ag::target_set::ExplicitEditTargetSet::default();
        targets.insert_authorized(
            "verification.txt".into(),
            ag::target_set::SOURCE_EXPLICIT_REFERENCE,
            None,
        );
        let read_ctx = ag::blocks::AgentReadContext::new(root, vec![])?;
        let identity = crate::types::ToolCallReceiptIdentityV1 {
            workspace_id: workspace.into(),
            session_id: session.clone(),
            turn_id: turn.clone(),
            tool_call_id: "isolated-effect".into(),
            tool_name: if resource_plan {
                "plan_workspace_operation"
            } else {
                "propose_edit"
            }
            .into(),
            capability_id: if resource_plan {
                "workspace_operation"
            } else {
                "document_edit"
            }
            .into(),
            grant_id: format!("isolated-{turn}"),
            attempt: 1,
        };
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let handle = app.clone();
        let source_turn = turn.clone();
        let task = if resource_plan {
            Some(
                ag::task::begin_turn(
                    &*state.db.lock().map_err(failure)?,
                    workspace,
                    &session,
                    None,
                )
                .map_err(failure)?,
            )
        } else {
            None
        };
        let worker_task = task.clone();
        let work = tauri::async_runtime::spawn(ag::capability_runtime::execute(
            state.clone(),
            identity,
            cancelled.clone(),
            Duration::from_millis(if cancel { 3000 } else { 80 }),
            move |state| {
                if resource_plan {
                    let request = crate::ipc::CommandRequest {
                        command: "ws.plan_operation".into(),
                        correlation_id: "isolated-resource-scan".into(),
                        params: serde_json::json!({"taskRunId":worker_task,"sourceTurnId":source_turn,"operations":[{"kind":"rename_resource","sourcePath":"verification.txt","targetPath":"never-renamed.txt","resourceKind":"file"}]}),
                    };
                    crate::workspace::routes::plan_workspace_operation_with_prepare(
                        &request,
                        state,
                        |ws, intents, task, turn, content| {
                            let prepared = crate::workspace::operation::prepare_operation(
                                ws, intents, task, turn, content,
                            )?;
                            started_tx.send(()).map_err(failure)?;
                            release_rx
                                .recv_timeout(Duration::from_secs(5))
                                .map_err(failure)?;
                            Ok(prepared)
                        },
                    )?;
                    return Ok(Vec::<crate::types::ToolCallReceiptReferenceV1>::new());
                }
                started_tx.send(()).map_err(failure)?;
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(failure)?;
                let result = ag::edit_runtime::ag_propose_edit(
                    state,
                    &handle,
                    &serde_json::json!({"path":"verification.txt", "operation":"replace_document", "content":"isolated proposal only"}),
                    &read_ctx,
                    None,
                    &targets,
                    &source_turn,
                    "isolated-effect",
                )?;
                Ok(result
                    .projections
                    .iter()
                    .map(|projection| crate::types::ToolCallReceiptReferenceV1 {
                        kind: crate::types::ToolCallReceiptReferenceKind::PendingDiff,
                        id: projection.diff_id.clone(),
                    })
                    .collect())
            },
            |refs| refs.clone(),
        ));
        let started = tauri::async_runtime::spawn_blocking(move || {
            started_rx.recv_timeout(Duration::from_secs(3))
        })
        .await
        .map_err(failure)?;
        if let Err(error) = started {
            let execution = work.await.map_err(failure)?;
            return Err(failure(format!(
                "effect fixture did not reach preparation: {error}; execution={execution:?}"
            )));
        }
        let terminal_started = Instant::now();
        if cancel {
            ag::turn_control::cancel_active_agent_turn(state, &session, &turn)?;
        }
        let error = work
            .await
            .map_err(failure)?
            .err()
            .ok_or_else(|| failure("expected bounded wait termination"))?;
        if error.code
            != if cancel {
                "AG_TURN_CANCELLED"
            } else {
                "AG_CAPABILITY_TIMEOUT"
            }
        {
            return Err(failure("wrong wait failure"));
        }
        let lock_available = state.db.try_lock().is_ok();
        if !lock_available {
            return Err(failure("slow preparation held shared DB lock"));
        }
        let emitter = ag::turn_events::TurnEventEmitter::without_channel(state, &turn);
        let latch = ag::turn::TerminalLatch::default();
        if cancel {
            ag::turn_events::emit_ag_turn_cancelled(&emitter, &latch, app, &turn);
        } else {
            ag::turn_events::emit_ag_turn_error(&emitter, &latch, app, &turn, &error);
        }
        let before = ag::turn_events::list_safe_events(
            &*state.db.lock().map_err(failure)?,
            workspace,
            &turn,
            0,
        )
        .map_err(failure)?;
        let terminal_while_scan_pending = lock_available
            && before.len() == (if cancel { 1 } else { 2 })
            && terminal_started.elapsed() < Duration::from_millis(500);
        let takeover_rejected = ag::turn_control::register_active_agent_turn(
            state,
            workspace,
            &session,
            "replacement",
            ag::turn_control::AgentTurnResumeKind::New,
        )
        .err()
        .is_some_and(|error| error.code == "AG_EFFECT_PENDING");
        drop(lease);
        release_tx.send(()).map_err(failure)?;
        let db = state.db.clone();
        let ws = workspace.to_string();
        let owner = session.clone();
        let id = turn.clone();
        let facts = tauri::async_runtime::spawn_blocking(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                let conn = db.lock().map_err(failure)?;
                let facts = ag::capability_runtime::facts(&conn, &ws, Some((&owner, &id)))
                    .map_err(failure)?;
                drop(conn);
                if facts.len() == 2 {
                    return Ok::<_, IpcError>(facts);
                }
                if Instant::now() >= deadline {
                    return Err(failure("late settlement missing"));
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
        .await
        .map_err(failure)??;
        let after = ag::turn_events::list_safe_events(
            &*state.db.lock().map_err(failure)?,
            workspace,
            &turn,
            0,
        )
        .map_err(failure)?;
        let terminal_unchanged = before.len() == (if cancel { 1 } else { 2 })
            && serde_json::to_value(&before).map_err(failure)?
                == serde_json::to_value(&after).map_err(failure)?;
        let settled = facts
            .iter()
            .find_map(|fact| fact.receipt.as_ref())
            .ok_or_else(|| failure("settled receipt missing"))?;
        let proposal_only = settled.effect_status
            == crate::types::ToolCallEffectStatus::ProposalCreated
            && settled.references.len() == 1;
        let file_unchanged = std::fs::read_to_string(root.join("verification.txt"))
            .map_err(failure)?
            == "isolated real file";
        let resource_rejected = resource_plan
            && settled.execution_status == crate::types::ToolCallExecutionStatus::Failed
            && settled
                .failure
                .as_ref()
                .is_some_and(|failure| failure.code == "AG_TURN_CANCELLED")
            && settled.references.is_empty();
        let no_resource_commit = if resource_plan {
            let conn = state.db.lock().map_err(failure)?;
            let plans: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM workspace_operation_plans WHERE workspace_id=?1",
                    [workspace],
                    |row| row.get(0),
                )
                .map_err(failure)?;
            let decisions: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM decision_requests WHERE task_run_id=?1",
                    [task.as_deref().unwrap_or("")],
                    |row| row.get(0),
                )
                .map_err(failure)?;
            plans == 0 && decisions == 0 && !root.join("never-renamed.txt").exists()
        } else {
            true
        };
        if !(takeover_rejected
            && terminal_unchanged
            && terminal_while_scan_pending
            && file_unchanged
            && no_resource_commit
            && if resource_plan {
                resource_rejected
            } else {
                proposal_only
            })
        {
            return Err(failure("effect contract guard mismatch"));
        }
        traces.push(serde_json::json!({"turnId":turn,"waitErrorCode":error.code,"takeoverRejected":takeover_rejected,
            "terminalUnchanged":terminal_unchanged,"terminalPersistedWhileScanPending":terminal_while_scan_pending,
            "caseKind":if resource_plan {"resource_plan"} else {"text_proposal"},"resourceRejectedAfterTerminal":resource_rejected,"noResourceCommit":no_resource_commit,
            "proposalOnly":proposal_only,"fileUnchanged":file_unchanged,"facts":facts}));
    }
    Ok(
        serde_json::json!({"scope":"owned_execution_port_real_txt_and_resource_plan_no_model_no_accept_no_save", "cases":traces}),
    )
}

/** @GOV
 * codes: BR-AG-OBS-001-API-AG-AG-TURN-061, BR-AG-STATE-001-API-AG-AG-TURN-061
 * type: API
 * chain: AG-TURN
 * rules: BR-AG-OBS-001, BR-AG-STATE-001
 * boundary: in=DevelopmentScenarioIdAndEventChannel | out=IsolatedProductionAgentTurnEvidence | does_not=useUserDatabaseOrGrantProductionPermission
 */
#[tauri::command]
pub(crate) async fn ag_verify_run<R: tauri::Runtime>(
    scenario: String,
    run_id: String,
    on_event: tauri::ipc::Channel<AgTurnEvent>,
    app: tauri::AppHandle<R>,
) -> Result<serde_json::Value, IpcError> {
    if ![
        "chat",
        "glob",
        "execution-budget-wrap",
        "action-handoff-stage",
        "action-handoff-plan",
        "tool-description-compatible",
        "tool-read-only-control-variants",
        "native-business-tools",
        "protocol-cumulative",
        "invalid-json",
        "truncated",
        "tool-failure",
        "page-contract-recovery",
        "page-contract-exhausted",
        "repeated-read-tools-blocked",
        "read-failure-same-batch",
        "read-failure-gate-denied",
        "read-failure-with-evidence",
        "read-failure-clarification",
        "protocol-corrects",
        "argument-corrects",
        "protocol-exhausted",
        "text-control-corrects",
        "argument-decision-shared-limit",
        "parameter-and-control-recover",
        "decision-native-replay-blocked",
        "text-correction-gate-denied",
        "text-correction-delay",
        "text-correction-incomplete",
        "text-correction-http402",
        "text-invalid-corrects",
        "text-incomplete-corrects",
        "text-empty-corrects",
        "text-protocol-exhausted",
        "tool-preamble",
        "tool-text-invalid-corrects",
        "after-read-text-corrects",
        "decision-text-corrects",
        "decision-text-replay-blocked",
        "text-decision-shared-limit",
        "text-decision-model-budget",
        "repeated-oversized-read",
        "oversized-read-recovery",
        "bounded-reading",
        "bounded-reading-four-pages",
        "gate-denied",
        "delay",
        "budget-initial",
        "budget-after-tool",
        "budget-recovery",
        "budget-recovery-delay",
        "decision-chain",
        "decision-duplicate",
        "decision-limit",
        "preview-guard",
        "session-memory",
        "effect-port",
    ]
    .contains(&scenario.as_str())
        || uuid::Uuid::parse_str(&run_id).is_err()
    {
        return Err(failure("Unsupported verification identity or scenario"));
    }
    let root = std::env::temp_dir().join(format!("binder-verification-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).map_err(failure)?;
    let sandbox = Sandbox(root);
    std::fs::write(sandbox.0.join("verification.txt"), "isolated real file").map_err(failure)?;
    if scenario.starts_with("bounded-reading") {
        let count = if scenario == "bounded-reading-four-pages" {
            1600
        } else {
            650
        };
        let html = (0..count).map(|index| format!("<p data-block-id='p-{index}' style='{}'>测试条目{index:04}：预算81000元，状态待评审。</p>", "color:black;".repeat(20))).collect::<String>();
        std::fs::write(sandbox.0.join("reading.html"), html).map_err(failure)?;
    }
    std::fs::write(sandbox.0.join("excluded.md"), "excluded fixture file").map_err(failure)?;
    if scenario.starts_with("read-failure-")
        || scenario == "repeated-oversized-read"
        || scenario == "oversized-read-recovery"
        || scenario == "repeated-read-tools-blocked"
    {
        std::fs::write(
            sandbox.0.join("oversized.txt"),
            (0..800)
                .map(|line| format!("oversized isolated read fixture line {line:04}\n"))
                .collect::<String>(),
        )
        .map_err(failure)?;
    }
    if scenario.starts_with("page-contract-") {
        std::fs::write(
            sandbox.0.join("pages.txt"),
            (0..25)
                .map(|i| format!("page fixture {i}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .map_err(failure)?;
    }
    let mut conn = rusqlite::Connection::open_in_memory().map_err(failure)?;
    crate::db::migrate(&mut conn).map_err(failure)?;
    let ws = crate::workspace::lifecycle::open_workspace(
        &mut conn,
        sandbox
            .0
            .to_str()
            .ok_or_else(|| failure("Invalid temporary path"))?,
    )
    .map_err(failure)?;
    let state = Arc::new(AppState {
        db: std::sync::Arc::new(Mutex::new(conn)),
        registry: crate::build_registry(),
        assist_terminals: std::sync::Arc::new(Mutex::new(HashMap::new())),
        active_agent_turns: Arc::new(Mutex::new(HashMap::new())),
    });
    let active_turn_id = Arc::new(Mutex::new(run_id.clone()));
    {
        let mut slot = RUN.lock().map_err(failure)?;
        if slot.is_some() {
            return Err(failure("A verification is already running"));
        }
        *slot = Some((run_id.clone(), state.clone(), active_turn_id.clone()));
    }
    let _lease = Lease(run_id.clone());
    if scenario == "effect-port" {
        let effect_trace =
            verify_effect_port(&state, &ws.workspace_id, &sandbox.0, &run_id, &app).await?;
        return Ok(
            serde_json::json!({"runId":run_id,"status":"effect_port_checked","events":[],"budgetChecks":[],
            "requestCounts":{"gate":0,"model":0},"decisionTrace":[],"modelToolMessages":[],"effectTrace":effect_trace,
            "environment":"development-isolated","realSubscriptionVerified":false}),
        );
    }
    if scenario == "preview-guard" {
        let preview_trace = verify_preview_guard(&state, &ws.workspace_id, &run_id)?;
        return Ok(serde_json::json!({
            "runId":run_id,"status":"preview_guard_checked","events":[],"budgetChecks":[],
            "requestCounts":{"gate":0,"model":0},"decisionTrace":[],"modelToolMessages":[],
            "previewTrace":preview_trace,"environment":"development-isolated","realSubscriptionVerified":false
        }));
    }
    let gateway = Gateway::start(&scenario)?;
    if scenario == "session-memory" {
        return verify_session_memory(
            &gateway,
            &state,
            &ws.workspace_id,
            &run_id,
            &active_turn_id,
            on_event,
            app,
        )
        .await;
    }
    let mut facts = serde_json::json!({"version":"agent-context-facts-v2","workspaceId":ws.workspace_id,"sessionId":run_id,"turnId":run_id,"contextRevision":1,"activeTarget":null,"openResources":[],"selections":[],"proposalFacts":[]});
    let reading_snapshot = if scenario.starts_with("bounded-reading") {
        facts["activeTarget"] = serde_json::json!({"relPath":"reading.html","loadSeq":1});
        facts["openResources"] = serde_json::json!([{ "relPath":"reading.html", "availability":"not_requested", "reasonCode":null, "snapshot":null }]);
        let content = std::fs::read_to_string(sandbox.0.join("reading.html")).map_err(failure)?;
        // Synthetic professional input, not a mock of the core, executor or budget.
        // This intentionally exercises the same active-HTML nine-tool manifest as production.
        let count = if scenario == "bounded-reading-four-pages" {
            1600
        } else {
            650
        };
        let blocks = (0..count).map(|index| {
            let id = format!("p-{index}");
            let text = format!("测试条目{index:04}：预算81000元，状态待评审。");
            ag::blocks::AgentEditableBlockSnapshot {
                block_id:id.clone(), block_index:index, block_type:"p".into(), node_type:"paragraph".into(), role:"content".into(),
                parent_block_id:None, depth:0, ordinal:index, text:text.clone(), attrs:serde_json::json!({"blockId":id}), marks:vec![],
                node_json:serde_json::json!({"type":"paragraph","attrs":{"blockId":id},"content":[{"type":"text","text":text}]}),
                fingerprint:format!("synthetic-paragraph-{index}"), capabilities:vec!["replace_node".into()],
            }
        }).collect();
        Some(ag::blocks::OpenDocSnapshot {
            rel_path: "reading.html".into(),
            content_hash: ag::context::sha256_text(&content),
            content,
            baseline: None,
            edit_format: "html_file".into(),
            content_format: "html".into(),
            source: "active_editor".into(),
            dirty: false,
            session_version: 1,
            captured_at: 1,
            block_snapshots: blocks,
            table_snapshots: vec![],
            image_snapshots: vec![],
        })
    } else {
        None
    };
    let context_channel = (scenario == "tool-failure"
        || scenario.starts_with("read-failure-")
        || scenario == "repeated-oversized-read"
        || scenario == "repeated-read-tools-blocked"
        || scenario.starts_with("page-contract-")
        || scenario == "oversized-read-recovery"
        || scenario.starts_with("bounded-reading"))
        .then(|| {
            let frozen = facts.clone();
            let reading_snapshot = reading_snapshot.clone();
            tauri::ipc::Channel::new(move |body| {
                let tauri::ipc::InvokeResponseBody::Json(json) = body else {
                    return Err(
                        std::io::Error::other("expected verification context request").into(),
                    );
                };
                let request: ag::turn_runtime::AgentContextRequest =
                    serde_json::from_str(&json).map_err(std::io::Error::other)?;
                let mut updated = frozen.clone();
                updated["contextRevision"] = request.context_revision.into();
                if let Some(snapshot) = &reading_snapshot {
                    updated["openResources"][0]["availability"] = "ready".into();
                    updated["openResources"][0]["snapshot"] = serde_json::json!({
                        "source":snapshot.source,"editFormat":snapshot.edit_format,"contentFormat":snapshot.content_format,
                        "dirty":snapshot.dirty,"baseline":snapshot.baseline,"sessionVersion":snapshot.session_version,"contentHash":snapshot.content_hash,
                    });
                }
                ag::turn_runtime::ag_supply_context(ag::turn_runtime::AgentContextResponse {
                    request_id: request.request_id,
                    facts: updated,
                    snapshots: reading_snapshot.clone().into_iter().collect(),
                })
                .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
                Ok(())
            })
        });
    let prompt = match scenario.as_str() {
        "action-handoff-plan" => {
            "准备将 verification.txt 改名为 renamed.txt，执行前通过确认卡询问。".into()
        }
        "chat" => "你好".to_string(),
        "bounded-reading" | "bounded-reading-four-pages" => {
            "完整读取 reading.html 的全部测试条目并分析，只读，不修改文件。".to_string()
        }
        "budget-initial" | "budget-recovery" | "budget-recovery-delay" => {
            "VERIFICATION_OVERSIZED_INPUT".repeat(12000)
        }
        _ => "列出工作区 txt 文件，或读取指定文件以验证错误路径".to_string(),
    };
    let mut decision_trace = Vec::new();
    let mut continuation: Option<(String, String)> = None;
    let segment_count = match scenario.as_str() {
        "action-handoff-plan" => 2,
        "read-failure-clarification" => 2,
        "decision-chain" => 3,
        "decision-duplicate" => 2,
        "decision-limit" => 7,
        "decision-text-corrects"
        | "decision-text-replay-blocked"
        | "text-decision-shared-limit" => 2,
        "argument-decision-shared-limit" | "decision-native-replay-blocked" => 2,
        "text-decision-model-budget" => 6,
        _ => 1,
    };
    for segment in 0..segment_count {
        let mut segment_facts = facts.clone();
        segment_facts["contextRevision"] = serde_json::json!(segment + 1);
        let clarification_resume = scenario == "read-failure-clarification" && segment > 0;
        if clarification_resume {
            segment_facts["contextRevision"] = serde_json::json!(20);
        }
        let history = if scenario == "bounded-reading-four-pages" {
            [
                ("user", "CAPACITY-USER：只读，不修改".into()),
                (
                    "assistant",
                    format!("CAPACITY-HEAD{}CAPACITY-TAIL", "史".repeat(100_000)),
                ),
            ]
            .into_iter()
            .map(|(role, content)| ag::context::HistTurn {
                workspace_id: ws.workspace_id.clone(),
                session_id: run_id.clone(),
                role: role.into(),
                content,
                rel_path: Some("reading.html".into()),
                turn_id: Some("synthetic-prior-turn".into()),
            })
            .collect()
        } else {
            vec![]
        };
        ag::tool_registry::with_verification_execution_policy(
            if scenario == "execution-budget-wrap" {
                ag::tool_registry::ExecutionBudgetPolicy {
                    max_model_calls: 4,
                    max_tool_calls: 1,
                    max_corrections: 2,
                    final_reserve: 2,
                }
            } else {
                ag::tool_registry::ExecutionBudgetPolicy::legacy()
            },
            ent::gateway::with_test_gateway(
                gateway.url.clone(),
                ag::turn_runtime::run_agent_turn(
                    run_id.clone(),
                    run_id.clone(),
                    Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                    if clarification_resume {
                        "只读取 oversized.txt 的第0块，不改文件。".into()
                    } else {
                        prompt.clone()
                    },
                    Some("fixture".into()),
                    Some("fixture-model".into()),
                    reading_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.rel_path.clone()),
                    None,
                    None,
                    vec![],
                    Some(segment_facts),
                    history,
                    vec![],
                    vec![],
                    vec![],
                    continuation.as_ref().map(|(task, _)| task.clone()),
                    continuation.as_ref().map(|(_, decision)| decision.clone()),
                    None,
                    Some(clarification_resume),
                    Some(segment > 0 && !clarification_resume),
                    on_event.clone(),
                    ag::turn_runtime::AgentContextChannel(
                        if segment == 0 || clarification_resume {
                            context_channel.clone()
                        } else {
                            None
                        },
                    ),
                    &state,
                    app.clone(),
                ),
            ),
        )
        .await?;
        if matches!(
            scenario.as_str(),
            "decision-chain"
                | "action-handoff-plan"
                | "decision-duplicate"
                | "decision-limit"
                | "decision-text-corrects"
                | "decision-text-replay-blocked"
                | "text-decision-shared-limit"
                | "argument-decision-shared-limit"
                | "decision-native-replay-blocked"
                | "text-decision-model-budget"
        ) && segment + 1 < segment_count
        {
            let (task_run_id, request_id, waiting_status) = {
                let conn = state.db.lock().map_err(failure)?;
                conn.query_row(
                    "SELECT d.task_run_id, d.request_id, r.status FROM decision_requests d JOIN task_runs r ON r.task_run_id=d.task_run_id WHERE r.workspace_id=?1 AND r.session_id=?2 AND d.status='proposed'",
                    rusqlite::params![&ws.workspace_id, &run_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
                )
                .map_err(failure)?
            };
            if waiting_status != "waiting" {
                return Err(failure(format!(
                    "DecisionRequest was not waiting: {waiting_status}"
                )));
            }
            let receipt = ag::task_runtime::route_resolve_decision(
                &crate::ipc::CommandRequest {
                    command: "ag.resolve_decision".into(),
                    correlation_id: run_id.clone(),
                    params: serde_json::json!({"requestId":request_id,"outcome":"confirm"}),
                },
                &state,
            )?;
            if receipt["already_resolved"] != false || receipt["task_run_id"] != task_run_id {
                return Err(failure(
                    "DecisionRequest resolution did not match the waiting TaskRun",
                ));
            }
            let duplicate = ag::task_runtime::route_resolve_decision(
                &crate::ipc::CommandRequest {
                    command: "ag.resolve_decision".into(),
                    correlation_id: run_id.clone(),
                    params: serde_json::json!({"requestId":request_id,"outcome":"confirm"}),
                },
                &state,
            )?;
            if duplicate["already_resolved"] != true || duplicate["task_run_id"] != task_run_id {
                return Err(failure(
                    "Repeated DecisionRequest resolution was not idempotent",
                ));
            }
            decision_trace.push(serde_json::json!({
                "requestId": request_id,
                "taskRunId": task_run_id,
                "waitingStatus": waiting_status,
                "outcome": "confirm",
                "alreadyResolved": false,
                "duplicateAlreadyResolved": true,
            }));
            continuation = Some((task_run_id, request_id));
        }
    }
    let follow_up_turn_id = if scenario == "budget-recovery" || scenario == "budget-recovery-delay"
    {
        let next_turn_id = uuid::Uuid::new_v4().to_string();
        *active_turn_id.lock().map_err(failure)? = next_turn_id.clone();
        let mut next_facts = facts.clone();
        next_facts["turnId"] = serde_json::json!(next_turn_id);
        next_facts["contextRevision"] = serde_json::json!(2);
        ag::tool_registry::with_verification_execution_policy(
            ag::tool_registry::ExecutionBudgetPolicy::legacy(),
            ent::gateway::with_test_gateway(
                gateway.url.clone(),
                ag::turn_runtime::run_agent_turn(
                    next_turn_id.clone(),
                    run_id.clone(),
                    Some(ag::turn::AGENT_EDIT_1_0_PROFILE.into()),
                    "你好".into(),
                    Some("fixture".into()),
                    Some("fixture-model".into()),
                    None,
                    None,
                    None,
                    vec![],
                    Some(next_facts),
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                    None,
                    None,
                    None,
                    None,
                    None,
                    on_event.clone(),
                    ag::turn_runtime::AgentContextChannel(None),
                    &state,
                    app.clone(),
                ),
            ),
        )
        .await?;
        Some(next_turn_id)
    } else {
        None
    };
    let conn = state.db.lock().map_err(failure)?;
    let status: String = conn
        .query_row(
            "SELECT status FROM agent_turns WHERE turn_id=?1",
            [&run_id],
            |row| row.get(0),
        )
        .map_err(failure)?;
    let mut events =
        ag::turn_events::list_safe_events(&conn, &ws.workspace_id, &run_id, 0).map_err(failure)?;
    let follow_up = if let Some(next_turn_id) = follow_up_turn_id.as_deref() {
        let (next_status, next_session_id): (String, String) = conn
            .query_row(
                "SELECT status, session_id FROM agent_turns WHERE turn_id=?1",
                [next_turn_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(failure)?;
        if next_session_id != run_id {
            return Err(failure(
                "Recovery AgentTurn did not retain its AgentSession",
            ));
        }
        events.extend(
            ag::turn_events::list_safe_events(&conn, &ws.workspace_id, next_turn_id, 0)
                .map_err(failure)?,
        );
        Some(
            serde_json::json!({"turnId":next_turn_id,"sessionId":next_session_id,"status":next_status}),
        )
    } else {
        None
    };
    let mut statement = conn
        .prepare("SELECT turn_id, payload_json FROM agent_turn_events WHERE workspace_id=?1 AND (turn_id=?2 OR turn_id=?3) AND kind='prompt_budget_checked' ORDER BY rowid ASC")
        .map_err(failure)?;
    let encoded_checks = statement
        .query_map(
            rusqlite::params![
                &ws.workspace_id,
                &run_id,
                follow_up_turn_id.as_deref().unwrap_or(&run_id)
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(failure)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(failure)?;
    let budget_checks = encoded_checks
        .into_iter()
        .map(|(turn_id, encoded)| {
            let stored: serde_json::Value = serde_json::from_str(&encoded).map_err(failure)?;
            for key in [
                "assemblyOrdinal",
                "nextModelCallOrdinal",
                "assemblyReason",
                "tokenLimit",
                "totalTokenLimit",
                "continuationReserveTokens",
                "measurement",
                "exceeded",
            ] {
                if stored.get(key).is_none() {
                    return Err(failure(format!("Missing budget evidence field: {key}")));
                }
            }
            Ok(serde_json::json!({
                "turnId": turn_id,
                "assemblyOrdinal": stored["assemblyOrdinal"],
                "nextModelCallOrdinal": stored["nextModelCallOrdinal"],
                "assemblyReason": stored["assemblyReason"],
                "tokenLimit": stored["tokenLimit"],
                "totalTokenLimit": stored["totalTokenLimit"],
                "continuationReserveTokens": stored["continuationReserveTokens"],
                "measurement": stored["measurement"],
                "exceeded": stored["exceeded"],
            }))
        })
        .collect::<Result<Vec<_>, IpcError>>()?;
    let request_paths = gateway.request_paths.lock().map_err(failure)?;
    let request_counts = serde_json::json!({
        "gate": request_paths.iter().filter(|path| path.as_str() == "/gate/check").count(),
        "model": request_paths.iter().filter(|path| path.as_str() == "/invoke/stream").count(),
    });
    let model_tool_messages = gateway.tool_messages.lock().map_err(failure)?.clone();
    let transport_trace = gateway.prompt_observations.lock().map_err(failure)?.clone();
    let mut rejected = conn.prepare("SELECT payload_json FROM agent_turn_events WHERE turn_id=?1 AND kind IN ('model_control_rejected','tool_parameter_contract_rejected') ORDER BY seq").map_err(failure)?;
    let protocol_trace = rejected.query_map([&run_id],|row|row.get::<_,String>(0)).map_err(failure)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(failure)?
        .into_iter().map(|raw| {
            let stored:serde_json::Value=serde_json::from_str(&raw).map_err(failure)?;
            // Export only format/error metadata, never control values or model text.
            Ok(serde_json::json!({"code":stored["code"],"prefixKind":stored["prefixKind"],"rejectionStage":stored["rejectionStage"],"correctionAttempt":stored["correctionAttempt"],"finalOnly":stored["finalOnly"],"protocolDiagnostic":stored["protocolDiagnostic"]}))
        }).collect::<Result<Vec<serde_json::Value>,IpcError>>()?;
    let mut observed = conn.prepare("SELECT payload_json FROM agent_turn_events WHERE turn_id=?1 AND kind='model_control_description_reconciled' ORDER BY seq").map_err(failure)?;
    let description_trace = observed
        .query_map([&run_id], |row| row.get::<_, String>(0))
        .map_err(failure)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(failure)?
        .into_iter()
        .map(|encoded| serde_json::from_str::<serde_json::Value>(&encoded).map_err(failure))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        serde_json::json!({"runId":run_id,"status":status,"followUp":follow_up,"events":events,"budgetChecks":budget_checks,"requestCounts":request_counts,"decisionTrace":decision_trace,"modelToolMessages":model_tool_messages,"protocolTrace":protocol_trace,"descriptionTrace":description_trace,"transportTrace":transport_trace,"environment":"development-isolated","realSubscriptionVerified":false}),
    )
}

#[tauri::command]
pub(crate) fn ag_verify_cancel(run_id: String) -> Result<(), IpcError> {
    let (state, active_turn_id) = RUN
        .lock()
        .map_err(failure)?
        .as_ref()
        .filter(|(id, _, _)| id == &run_id)
        .map(|(_, state, turn_id)| (state.clone(), turn_id.clone()))
        .ok_or_else(|| failure("Verification is not running"))?;
    let turn_id = active_turn_id.lock().map_err(failure)?.clone();
    super::turn_control::cancel_active_agent_turn(&state, &run_id, &turn_id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    static TEST_RUN: Mutex<()> = Mutex::new(());

    // covers: BR-SYS-STATE-008, BR-AG-DATA-001
    #[test]
    fn isolated_preview_probe_uses_real_checkpoint_guards() {
        let _serial = TEST_RUN.lock().unwrap();
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let channel = tauri::ipc::Channel::new(|_| Ok(()));
        let receipt = tauri::async_runtime::block_on(ag_verify_run(
            "preview-guard".into(),
            id,
            channel,
            app.handle().clone(),
        ))
        .unwrap();
        assert_eq!(receipt["status"], "preview_guard_checked");
        assert_eq!(receipt["previewTrace"]["staleV2ContinuationRejected"], true);
        assert_eq!(receipt["previewTrace"]["matchingV1ContinuationOnce"], true);
        assert_eq!(
            receipt["previewTrace"]["duplicateContinuationRejected"],
            true
        );
        assert_eq!(receipt["requestCounts"]["model"], 0);
        assert!(RUN.lock().unwrap().is_none());
        let effect_receipt = tauri::async_runtime::block_on(ag_verify_run(
            "effect-port".into(),
            uuid::Uuid::new_v4().to_string(),
            tauri::ipc::Channel::new(|_| Ok(())),
            app.handle().clone(),
        ))
        .unwrap();
        assert_eq!(effect_receipt["status"], "effect_port_checked");
        assert_eq!(effect_receipt["requestCounts"]["model"], 0);
        assert_eq!(
            effect_receipt["effectTrace"]["cases"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        for case in effect_receipt["effectTrace"]["cases"].as_array().unwrap() {
            assert_eq!(case["terminalUnchanged"], true);
            assert_eq!(case["fileUnchanged"], true);
            assert_eq!(case["takeoverRejected"], true);
            assert_eq!(case["terminalPersistedWhileScanPending"], true);
            if case["caseKind"] == "resource_plan" {
                assert_eq!(case["resourceRejectedAfterTerminal"], true);
                assert_eq!(case["noResourceCommit"], true);
                assert_eq!(case["proposalOnly"], false);
            } else {
                assert_eq!(case["proposalOnly"], true);
            }
        }
        assert!(RUN.lock().unwrap().is_none());
    }

    // covers: BR-AG-STATE-001, BR-AG-STATE-002, BR-AG-OBS-001, BR-AG-DATA-005, BR-AG-PERSIST-001, BR-SYS-DATA-007, BR-SYS-STATE-007, BR-SYS-STATE-008
    #[test]
    fn development_scenarios_execute_real_core_and_release_isolated_state() {
        let _serial = TEST_RUN.lock().unwrap();
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        for scenario in [
            "chat",
            "glob",
            "execution-budget-wrap",
            "action-handoff-stage",
            "action-handoff-plan",
            "tool-description-compatible",
            "tool-read-only-control-variants",
            "native-business-tools",
            "protocol-cumulative",
            "invalid-json",
            "truncated",
            "tool-failure",
            "page-contract-recovery",
            "page-contract-exhausted",
            "repeated-read-tools-blocked",
            "read-failure-same-batch",
            "read-failure-gate-denied",
            "read-failure-with-evidence",
            "read-failure-clarification",
            "protocol-corrects",
            "argument-corrects",
            "protocol-exhausted",
            "text-control-corrects",
            "argument-decision-shared-limit",
            "parameter-and-control-recover",
            "decision-native-replay-blocked",
            "text-correction-gate-denied",
            "text-correction-delay",
            "text-correction-incomplete",
            "text-correction-http402",
            "text-invalid-corrects",
            "text-incomplete-corrects",
            "text-empty-corrects",
            "text-protocol-exhausted",
            "tool-preamble",
            "tool-text-invalid-corrects",
            "after-read-text-corrects",
            "decision-text-corrects",
            "decision-text-replay-blocked",
            "text-decision-shared-limit",
            "text-decision-model-budget",
            "repeated-oversized-read",
            "oversized-read-recovery",
            "bounded-reading",
            "bounded-reading-four-pages",
            "gate-denied",
            "delay",
            "budget-initial",
            "budget-after-tool",
            "budget-recovery",
            "budget-recovery-delay",
            "decision-chain",
            "decision-duplicate",
            "decision-limit",
            "session-memory",
        ] {
            let id = uuid::Uuid::new_v4().to_string();
            let projected = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
            let captured = projected.clone();
            let channel = tauri::ipc::Channel::new(move |body| {
                if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                    captured
                        .lock()
                        .unwrap()
                        .push(serde_json::from_str(&json).unwrap());
                }
                Ok(())
            });
            let canceller =
                (scenario == "delay" || scenario == "budget-recovery-delay" || scenario == "text-correction-delay").then(|| {
                    let id = id.clone();
                    let recovery_delay = scenario == "budget-recovery-delay";
                    let correction_delay = scenario == "text-correction-delay";
                    std::thread::spawn(move || {
                        let deadline = Instant::now() + Duration::from_secs(10);
                        while Instant::now() < deadline {
                            let started = RUN.lock().unwrap().as_ref().is_some_and(
                                |(key, state, turn_id)| {
                                    key == &id
                                        && (!recovery_delay || *turn_id.lock().unwrap() != id)
                                        && !state.active_agent_turns.lock().unwrap().is_empty()
                                        && (!correction_delay || state.db.lock().unwrap().query_row(
                                            "SELECT COUNT(*) FROM agent_turn_events WHERE turn_id=?1 AND kind='provider_started'",
                                            [&id], |row|row.get::<_,usize>(0)).unwrap()>=2)
                                },
                            );
                            if started {
                                std::thread::sleep(Duration::from_millis(200));
                                ag_verify_cancel(id).unwrap();
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        panic!("verification did not start");
                    })
                });
            let receipt = tauri::async_runtime::block_on(ag_verify_run(
                scenario.into(),
                id,
                channel,
                app.handle().clone(),
            ))
            .unwrap();
            if let Some(worker) = canceller {
                worker.join().unwrap();
            }
            assert!(RUN.lock().unwrap().is_none());
            assert_eq!(
                receipt["events"],
                serde_json::to_value(&*projected.lock().unwrap()).unwrap()
            );
            let events = receipt["events"].as_array().unwrap();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| {
                        serde_json::from_value::<AgTurnEvent>((*event).clone())
                            .unwrap()
                            .event
                            .terminal_status()
                            .is_some()
                    })
                    .count(),
                if scenario == "session-memory" {
                    5
                } else if scenario == "budget-recovery" || scenario == "budget-recovery-delay" {
                    2
                } else {
                    1
                },
                "exactly one real terminal event per AgentTurn"
            );
            let tools: Vec<_> = events
                .iter()
                .filter(|event| event["type"] == "tool_result")
                .collect();
            for tool in &tools {
                let execution = &tool["executionReceipt"];
                assert_eq!(execution["version"], "tool-call-receipt-v1");
                assert_eq!(execution["turnId"], tool["turnId"]);
                assert_eq!(execution["toolCallId"], tool["toolCallId"]);
                assert_eq!(execution["toolName"], tool["toolName"]);
                if tool["success"] == true && tool["toolName"] == "propose_decision" {
                    assert_eq!(execution["completionStatus"], "waiting");
                    assert_eq!(execution["effectStatus"], "not_started");
                    assert_eq!(execution["nextAction"], "await_user");
                    let request_id = execution["references"][0]["id"].as_str().unwrap();
                    assert!(events
                        .iter()
                        .any(|event| event["type"] == "decision_request_proposed"
                            && event["requestId"] == request_id));
                }
            }
            let budget_checks = receipt["budgetChecks"].as_array().unwrap();
            for check in budget_checks {
                let measurement = &check["measurement"];
                let layers = measurement["messageLayers"].as_object().unwrap();
                let layer_chars: u64 = layers.values().map(|value| value.as_u64().unwrap()).sum();
                let serialized_chars = measurement["serializedChars"].as_u64().unwrap();
                assert_eq!(
                    serialized_chars,
                    layer_chars
                        + measurement["messageFramingChars"].as_u64().unwrap()
                        + measurement["toolSchemaChars"].as_u64().unwrap()
                );
                assert_eq!(
                    measurement["estimatedTokens"].as_u64().unwrap(),
                    serialized_chars.div_ceil(4)
                );
                assert_eq!(
                    check["tokenLimit"].as_u64().unwrap()
                        + check["continuationReserveTokens"].as_u64().unwrap(),
                    check["totalTokenLimit"].as_u64().unwrap()
                );
            }
            assert!(!receipt.to_string().contains("VERIFICATION_OVERSIZED_INPUT"));
            assert!(!receipt
                .to_string()
                .contains("VERIFICATION_OVERSIZED_PATTERN"));
            match scenario {
                "native-business-tools" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 5);
                    assert_eq!(
                        tools
                            .iter()
                            .filter(|event| event["success"] == true)
                            .count(),
                        3
                    );
                    for tool in tools.iter().filter(|event| event["success"] == false) {
                        assert_eq!(tool["executionReceipt"]["executionStatus"], "rejected");
                        assert_eq!(tool["executionReceipt"]["effectStatus"], "not_started");
                    }
                    assert_eq!(receipt["requestCounts"]["model"], 5);
                    let rejects = receipt["protocolTrace"].as_array().unwrap();
                    assert_eq!(rejects.len(), 2);
                    assert_eq!(rejects[0]["protocolDiagnostic"]["fieldPath"], "_binderTurn");
                    assert_eq!(rejects[1]["protocolDiagnostic"]["fieldPath"], "$control");
                    let returned = receipt["modelToolMessages"].as_array().unwrap();
                    assert_eq!(returned.len(), 3);
                    assert!(returned.iter().all(|row| {
                        row["toolCallId"] != "verification-native-missing"
                            && row["toolCallId"] != "verification-native-type"
                    }));
                    assert!(returned.iter().any(|row| row["toolCallId"]
                        == "verification-native-glob"
                        && row["content"]
                            .as_str()
                            .unwrap()
                            .contains("verification.txt")));
                }
                "tool-description-compatible" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 3);
                    assert!(tools.iter().all(|tool| tool["success"] == true));
                    assert_eq!(receipt["requestCounts"]["model"], 2);
                    assert!(receipt["protocolTrace"].as_array().unwrap().is_empty());
                    assert_eq!(
                        receipt["descriptionTrace"][0]["observations"]
                            .as_array()
                            .unwrap()
                            .len(),
                        2
                    );
                    assert!(!receipt["descriptionTrace"].to_string().contains("private"));
                }
                "tool-read-only-control-variants" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 2);
                    assert!(tools.iter().all(|tool| tool["success"] == true));
                    assert!(receipt["protocolTrace"].as_array().unwrap().is_empty());
                    assert_eq!(
                        receipt["descriptionTrace"][0]["observations"][0]["controlFields"],
                        serde_json::json!(["intent.requiredContext"])
                    );
                    assert!(!receipt["descriptionTrace"]
                        .to_string()
                        .contains("PRIVATE_BLOCK_HINT"));
                }
                "protocol-cumulative" => {
                    assert_eq!(receipt["status"], "error");
                    assert_eq!(receipt["requestCounts"]["model"], 4);
                    assert_eq!(
                        tools.iter().filter(|tool| tool["success"] == true).count(),
                        1
                    );
                    for (index, row) in receipt["protocolTrace"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .enumerate()
                    {
                        assert_eq!(row["correctionAttempt"], index + 1);
                        assert!(row["protocolDiagnostic"]["reasonCode"].is_string());
                        assert_eq!(row["protocolDiagnostic"]["controlSource"], "tool[0]");
                    }
                    assert_eq!(receipt["protocolTrace"].as_array().unwrap().len(), 3);
                    assert!(events
                        .iter()
                        .any(|e| e["code"] == "AG_MODEL_CORRECTION_LIMIT"
                            && e["diagnostic"]["category"] == "model_protocol"));
                }
                "action-handoff-plan" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["model"], 3);
                    assert_eq!(tools.len(), 1);
                    assert_eq!(tools[0]["toolName"], "plan_workspace_operation");
                    assert_eq!(tools[0]["success"], true);
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e["type"] == "decision_request_proposed")
                            .count(),
                        1
                    );
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e["type"] == "decision_request_resolved")
                            .count(),
                        1
                    );
                    assert!(!serde_json::to_string(events)
                        .unwrap()
                        .contains("UNDELIVERED_PLAN_PROMISE"));
                }
                "execution-budget-wrap" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 1);
                    assert_eq!(tools[0]["success"], true);
                    assert_eq!(receipt["requestCounts"]["model"], 2);
                    assert_eq!(receipt["transportTrace"][1]["nativeToolCount"], 0);
                    assert!(events
                        .iter()
                        .any(|e| e["type"] == "done" && e["stopReason"] == "budget_exhausted"));
                }
                "action-handoff-stage" => {
                    assert_eq!(receipt["status"], "completed");
                    assert!(tools.is_empty());
                    assert_eq!(receipt["requestCounts"]["model"], 2);
                    assert!(!serde_json::to_string(events)
                        .unwrap()
                        .contains("UNDELIVERED_PLAN_PROMISE"));
                    assert_eq!(
                        receipt["protocolTrace"][0]["code"],
                        "AG_WORKSPACE_PLAN_TOOL_REQUIRED"
                    );
                    assert!(events
                        .iter()
                        .any(|e| e["type"] == "done" && e["stopReason"] == "report_only"));
                }
                "argument-decision-shared-limit" | "decision-native-replay-blocked" => {
                    assert_eq!(receipt["status"], "error");
                    let attempts = if scenario == "argument-decision-shared-limit" {
                        5
                    } else {
                        4
                    };
                    assert_eq!(receipt["requestCounts"]["model"], attempts);
                    assert_eq!(receipt["requestCounts"]["gate"], attempts);
                    assert_eq!(tools.len(), 2);
                    assert_eq!(
                        tools.iter().filter(|tool| tool["success"] == true).count(),
                        1
                    );
                    assert!(events
                        .iter()
                        .any(|e| e["type"] == "error" && e["code"] == "AG_MODEL_CORRECTION_LIMIT"));
                    let rejects = receipt["protocolTrace"].as_array().unwrap();
                    assert_eq!(
                        rejects.len(),
                        if scenario == "argument-decision-shared-limit" {
                            4
                        } else {
                            3
                        }
                    );
                    for (index, rejected) in rejects.iter().enumerate() {
                        let expected = if scenario == "argument-decision-shared-limit" && index > 0
                        {
                            index
                        } else {
                            index + 1
                        };
                        assert_eq!(rejected["correctionAttempt"], expected);
                    }
                    assert_eq!(receipt["decisionTrace"].as_array().unwrap().len(), 1);
                }
                "parameter-and-control-recover" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["model"], 5);
                    assert_eq!(receipt["requestCounts"]["gate"], 5);
                    assert_eq!(tools.len(), 3);
                    assert!(tools[..2].iter().all(|tool| tool["success"] == false));
                    assert_eq!(tools[2]["success"], true);
                    let rejects = receipt["protocolTrace"].as_array().unwrap();
                    assert_eq!(rejects.len(), 3);
                    assert_eq!(rejects[0]["correctionAttempt"], 1);
                    assert_eq!(rejects[1]["correctionAttempt"], 2);
                    assert_eq!(rejects[2]["correctionAttempt"], 1);
                    assert!(events.iter().any(|e| e["type"] == "done"));
                }
                "text-correction-gate-denied"
                | "text-correction-delay"
                | "text-correction-incomplete"
                | "text-correction-http402" => {
                    let (status, models, terminal_code) = match scenario {
                        "text-correction-gate-denied" => {
                            ("gate_rejected", 1, Some("AG_GATE_DENIED"))
                        }
                        "text-correction-delay" => ("cancelled", 2, None),
                        "text-correction-incomplete" => {
                            ("error", 2, Some("AG_GW_STREAM_INCOMPLETE"))
                        }
                        _ => ("error", 2, Some("HTTP_402")),
                    };
                    assert_eq!(receipt["status"], status, "{scenario}");
                    assert_eq!(receipt["requestCounts"]["gate"], 2, "{scenario}");
                    assert_eq!(receipt["requestCounts"]["model"], models, "{scenario}");
                    assert!(tools.is_empty());
                    if let Some(code) = terminal_code {
                        assert!(events
                            .iter()
                            .any(|e| e["type"] == "error" && e["code"] == code));
                    }
                    assert!(events.iter().all(|e| e["type"] != "chunk"
                        || !e["text"]
                            .as_str()
                            .unwrap()
                            .contains("PRIVATE_UNVALIDATED_TEXT")));
                }
                "text-control-corrects"
                | "text-invalid-corrects"
                | "text-incomplete-corrects"
                | "text-empty-corrects"
                | "tool-preamble"
                | "tool-text-invalid-corrects"
                | "after-read-text-corrects"
                | "decision-text-corrects"
                | "text-protocol-exhausted"
                | "decision-text-replay-blocked"
                | "text-decision-shared-limit"
                | "text-decision-model-budget" => {
                    let (expected_status, expected_calls, expected_tools) = match scenario {
                        "tool-preamble" => ("completed", 2, 1),
                        "tool-text-invalid-corrects"
                        | "after-read-text-corrects"
                        | "decision-text-corrects" => ("completed", 3, 1),
                        "text-protocol-exhausted" => ("error", 3, 0),
                        "decision-text-replay-blocked" | "text-decision-shared-limit" => {
                            ("error", 4, 1)
                        }
                        "text-decision-model-budget" => ("error", 6, 5),
                        _ => ("completed", 2, 0),
                    };
                    assert_eq!(receipt["status"], expected_status, "{scenario}");
                    assert_eq!(
                        receipt["requestCounts"]["model"], expected_calls,
                        "{scenario}"
                    );
                    assert_eq!(
                        receipt["requestCounts"]["gate"], expected_calls,
                        "{scenario}"
                    );
                    assert_eq!(
                        tools.len(),
                        expected_tools,
                        "{scenario}: invalid batches never execute"
                    );
                    assert!(!events.iter().any(|e| e["type"] == "chunk"
                        && e["text"]
                            .as_str()
                            .is_some_and(|text| text.contains("PRIVATE_UNVALIDATED_TEXT"))));
                    assert!(tools.iter().all(|tool| tool["success"] == true));
                    let rejects = receipt["protocolTrace"].as_array().unwrap();
                    let expected_rejections = match scenario {
                        "tool-preamble" => 0,
                        "text-protocol-exhausted"
                        | "decision-text-replay-blocked"
                        | "text-decision-shared-limit" => 3,
                        _ => 1,
                    };
                    assert_eq!(rejects.len(), expected_rejections, "{scenario}");
                    for (index, rejected) in rejects.iter().enumerate() {
                        assert_eq!(rejected["correctionAttempt"], index + 1);
                        assert_eq!(rejected["rejectionStage"], "response_completion");
                    }
                    assert!(!receipt.to_string().contains("PRIVATE_UNVALIDATED_TEXT"));
                    if scenario == "after-read-text-corrects" {
                        assert!(!tools[0]["evidenceIds"].as_array().unwrap().is_empty());
                        assert!(events.iter().any(|e| e["type"] == "project_report"
                            && e["report"]["evidenceThroughSeq"].as_u64().unwrap_or(0) > 0));
                    }
                    if expected_status == "error" {
                        let expected_code = if scenario == "text-decision-model-budget" {
                            "AG_TOOL_LOOP_LIMIT"
                        } else {
                            "AG_MODEL_CORRECTION_LIMIT"
                        };
                        assert!(events
                            .iter()
                            .any(|e| e["type"] == "error" && e["code"] == expected_code));
                    }
                }
                "decision-duplicate" | "decision-limit" => {
                    assert_eq!(receipt["status"], "error");
                    let expected = if scenario == "decision-duplicate" {
                        2
                    } else {
                        6
                    };
                    assert_eq!(receipt["requestCounts"]["model"], expected);
                    assert_eq!(receipt["requestCounts"]["gate"], expected);
                    let code = if scenario == "decision-duplicate" {
                        "AG_MODEL_TOOL_ID_INVALID"
                    } else {
                        "AG_TOOL_LOOP_LIMIT"
                    };
                    assert!(receipt["events"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|event| event["type"] == "error" && event["code"] == code));
                    assert_eq!(
                        receipt["decisionTrace"].as_array().unwrap().len(),
                        if scenario == "decision-duplicate" {
                            1
                        } else {
                            6
                        }
                    );
                }
                "session-memory" => {
                    assert_eq!(receipt["status"], "session_memory_checked");
                    assert_eq!(receipt["requestCounts"]["gate"], 6);
                    assert_eq!(receipt["requestCounts"]["model"], 6);
                    assert_eq!(receipt["promptTrace"].as_array().unwrap().len(), 6);
                    assert_eq!(receipt["promptTrace"][2]["selectedMemory"], true);
                    assert_eq!(receipt["promptTrace"][3]["selectedMemory"], false);
                    assert_eq!(receipt["promptTrace"][5]["oldHistory"], false);
                    assert_eq!(
                        receipt["sessionTrace"]["crossSessionHistoryRejectedBeforeGate"],
                        true
                    );
                    assert_eq!(receipt["sessionTrace"]["waitingTaskStatus"], "waiting");
                    assert_eq!(receipt["sessionTrace"]["memoryEntryCount"], 0);
                    assert!(tools
                        .iter()
                        .any(|tool| tool["toolName"] == "propose_decision"));
                }
                "chat" => {
                    assert_eq!(receipt["status"], "completed");
                    assert!(tools.is_empty());
                }
                "glob" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 1);
                    assert_eq!(tools[0]["success"], true);
                    assert_eq!(tools[0]["result"], "workspace_glob 执行成功");
                    let messages = receipt["modelToolMessages"].as_array().unwrap();
                    assert_eq!(messages.len(), 1);
                    assert_eq!(messages[0]["toolCallId"], tools[0]["toolCallId"]);
                    let content: serde_json::Value =
                        serde_json::from_str(messages[0]["content"].as_str().unwrap()).unwrap();
                    assert!(content["toolResult"]
                        .to_string()
                        .contains("verification.txt"));
                    assert_eq!(content["evidenceIds"], tools[0]["evidenceIds"]);
                    assert_eq!(content["executionReceipt"], tools[0]["executionReceipt"]);
                    assert_eq!(
                        tools[0]["executionReceipt"]["completionStatus"],
                        "completed"
                    );
                    assert_eq!(
                        tools[0]["executionReceipt"]["effectStatus"],
                        "not_applicable"
                    );
                    assert!(!content["toolResult"].to_string().contains("excluded.md"));
                    assert!(!tools[0]["evidenceIds"].as_array().unwrap().is_empty());
                }
                "tool-failure" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 1);
                    assert_eq!(tools[0]["success"], false);
                    assert_eq!(tools[0]["diagnostic"]["observedAt"], "tool_execution");
                    assert!(!tools[0]["result"].as_str().unwrap().contains("AG_CONTEXT_"));
                    let messages = receipt["modelToolMessages"].as_array().unwrap();
                    assert_eq!(messages.len(), 1);
                    assert_eq!(messages[0]["toolCallId"], tools[0]["toolCallId"]);
                    assert!(!messages[0]["content"]
                        .as_str()
                        .unwrap()
                        .contains("AG_CONTEXT_"));
                }
                "protocol-corrects" | "argument-corrects" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["gate"], 3);
                    assert_eq!(receipt["requestCounts"]["model"], 3);
                    assert_eq!(tools.len(), 2);
                    assert_eq!(tools[0]["success"], false);
                    assert_eq!(tools[0]["executionReceipt"]["executionStatus"], "rejected");
                    assert_eq!(tools[1]["success"], true);
                    assert_eq!(tools[1]["executionReceipt"]["executionStatus"], "succeeded");
                    assert!(tools[0]["evidenceIds"].as_array().unwrap().is_empty());
                    assert!(!receipt.to_string().contains("oversized.txt"));
                }
                "protocol-exhausted" => {
                    assert_eq!(receipt["status"], "error");
                    assert_eq!(receipt["requestCounts"]["gate"], 3);
                    assert_eq!(receipt["requestCounts"]["model"], 3);
                    assert_eq!(tools.len(), 2);
                    assert!(tools
                        .iter()
                        .all(|tool| tool["executionReceipt"]["executionStatus"] == "rejected"));
                    assert!(events.iter().any(|event| event["type"] == "error"
                        && event["code"] == "AG_MODEL_CORRECTION_LIMIT"));
                }
                "repeated-oversized-read" | "repeated-read-tools-blocked" => {
                    assert_eq!(receipt["status"], "completed");
                    let expected = if scenario == "repeated-read-tools-blocked" {
                        4
                    } else {
                        3
                    };
                    assert_eq!(receipt["requestCounts"]["gate"], expected);
                    assert_eq!(receipt["requestCounts"]["model"], expected);
                    assert_eq!(tools.len(), 2);
                    assert!(tools.iter().all(|tool| tool["success"] == false
                        && tool["executionReceipt"]["failure"]["code"]
                            == "AG_TOOL_RESULT_TOO_LARGE"
                        && tool["evidenceIds"].as_array().unwrap().is_empty()));
                    assert!(!events.iter().any(|event| event["type"] == "error"));
                    let requests = receipt["transportTrace"].as_array().unwrap();
                    assert!(requests[0]["nativeToolCount"].as_u64().unwrap() > 0);
                    assert!(requests[2..]
                        .iter()
                        .all(|request| request["nativeToolCount"] == 0));
                    assert!(events
                        .iter()
                        .find(|event| event["type"] == "project_report")
                        .unwrap()["report"]["readItems"]
                        .as_array()
                        .unwrap()
                        .is_empty());
                    if scenario == "repeated-read-tools-blocked" {
                        assert_eq!(receipt["protocolTrace"][0]["finalOnly"], true);
                    }
                    assert!(!receipt.to_string().contains("line 0799"));
                }
                "read-failure-clarification" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["model"], 5);
                    assert_eq!(receipt["requestCounts"]["gate"], 5);
                    assert_eq!(tools.len(), 3);
                    assert!(tools[..2].iter().all(|tool| tool["success"] == false));
                    assert_eq!(tools[2]["success"], true);
                    assert!(events
                        .iter()
                        .any(|event| event["type"] == "clarification_required"));
                    assert_eq!(receipt["transportTrace"][2]["nativeToolCount"], 0);
                    assert!(
                        receipt["transportTrace"][3]["nativeToolCount"]
                            .as_u64()
                            .unwrap()
                            > 0
                    );
                }
                "read-failure-same-batch"
                | "read-failure-gate-denied"
                | "read-failure-with-evidence" => {
                    let denied = scenario == "read-failure-gate-denied";
                    let prior = scenario == "read-failure-with-evidence";
                    assert_eq!(
                        receipt["status"],
                        if denied { "gate_rejected" } else { "completed" }
                    );
                    assert_eq!(receipt["requestCounts"]["model"], if prior { 4 } else { 2 });
                    assert_eq!(
                        receipt["requestCounts"]["gate"],
                        if prior {
                            4
                        } else if denied {
                            3
                        } else {
                            2
                        }
                    );
                    assert_eq!(tools.len(), if denied { 2 } else { 3 });
                    if prior {
                        assert_eq!(tools[0]["success"], true);
                        assert!(!tools[0]["evidenceIds"].as_array().unwrap().is_empty());
                        let report = events
                            .iter()
                            .find(|event| event["type"] == "project_report")
                            .unwrap();
                        assert!(report["report"]["readItems"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|item| item["relPath"] == "verification.txt"));
                    } else {
                        assert!(tools.iter().all(|tool| tool["success"] == false
                            && tool["evidenceIds"].as_array().unwrap().is_empty()));
                    }
                    if scenario == "read-failure-same-batch" {
                        assert_eq!(tools[2]["executionReceipt"]["executionStatus"], "rejected");
                        assert_eq!(tools[2]["executionReceipt"]["effectStatus"], "not_started");
                    }
                }
                "page-contract-recovery" | "page-contract-exhausted" => {
                    let recovery = scenario == "page-contract-recovery";
                    assert_eq!(
                        receipt["status"],
                        if recovery { "completed" } else { "error" }
                    );
                    assert_eq!(
                        receipt["requestCounts"]["model"],
                        if recovery { 5 } else { 4 }
                    );
                    assert_eq!(
                        receipt["requestCounts"]["gate"],
                        if recovery { 5 } else { 4 }
                    );
                    assert_eq!(tools.len(), if recovery { 4 } else { 3 });
                    assert_eq!(tools[0]["success"], true);
                    assert_eq!(tools[1]["executionReceipt"]["executionStatus"], "rejected");
                    assert_eq!(
                        tools[1]["executionReceipt"]["failure"]["code"],
                        "AG_READ_CURSOR_INVALID"
                    );
                    assert_eq!(tools[1]["diagnostic"]["category"], "model_protocol");
                    assert_eq!(tools[1]["diagnostic"]["observedAt"], "tool_validation");
                    assert!(tools[1]["evidenceIds"].as_array().unwrap().is_empty());
                    if recovery {
                        assert!(tools[2..].iter().all(|tool| tool["success"] == true));
                        let replies = receipt["modelToolMessages"].as_array().unwrap();
                        let page: serde_json::Value = serde_json::from_str(
                            replies.last().unwrap()["content"].as_str().unwrap(),
                        )
                        .unwrap();
                        assert_eq!(page["toolResult"]["offset"], 20);
                        assert_eq!(page["toolResult"]["blocks"].as_array().unwrap().len(), 5);
                    } else {
                        assert!(events
                            .iter()
                            .any(|event| event["code"] == "AG_MODEL_CORRECTION_LIMIT"));
                    }
                }
                "bounded-reading" | "bounded-reading-four-pages" => {
                    let count = if scenario == "bounded-reading-four-pages" {
                        1600
                    } else {
                        650
                    };
                    let page_count = if scenario == "bounded-reading-four-pages" {
                        4
                    } else {
                        2
                    };
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["gate"], page_count + 1);
                    assert_eq!(receipt["requestCounts"]["model"], page_count + 1);
                    assert_eq!(tools.len(), page_count);
                    assert!(tools.iter().all(|tool| tool["success"] == true));
                    let messages = receipt["modelToolMessages"].as_array().unwrap();
                    let pages = messages
                        .iter()
                        .map(|message| {
                            serde_json::from_str::<serde_json::Value>(
                                message["content"].as_str().unwrap(),
                            )
                            .unwrap()["toolResult"]
                                .clone()
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(pages.len(), page_count);
                    for pair in pages.windows(2) {
                        assert_eq!(pair[0]["next_offset"], pair[1]["offset"]);
                        assert_eq!(pair[0]["version_hash"], pair[1]["version_hash"]);
                    }
                    assert_eq!(pages[0]["has_more"], true);
                    assert_eq!(pages[page_count - 1]["has_more"], false);
                    assert!(pages.iter().all(|page| page["complete"] == false));
                    let text = pages
                        .iter()
                        .map(|page| page["content"].as_str().unwrap())
                        .collect::<String>();
                    let expected = (0..count)
                        .map(|index| format!("测试条目{index:04}：预算81000元，状态待评审。\n"))
                        .collect::<String>();
                    assert_eq!(text, expected);
                    assert!(!text.contains("style=") && !text.contains("node_json"));
                    let report = events
                        .iter()
                        .find(|event| event["type"] == "project_report")
                        .unwrap();
                    assert!(!report["report"]["readItems"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| item["coverage"] == "full_document"));
                    if scenario == "bounded-reading-four-pages" {
                        let trace = receipt["transportTrace"].as_array().unwrap();
                        assert_eq!(trace.len(), 5);
                        assert!(trace.iter().all(|row| row["historyUserVisible"] == true
                            && row["historyHeadVisible"] == true
                            && row["historyTailVisible"] == true));
                        assert!(trace.iter().any(|row| row["historyExcerpted"] == true));
                        assert!(budget_checks.iter().all(|check| check["exceeded"] == false));
                        eprintln!(
                            "[AG-AG-TURN] history_capacity_probe: {}",
                            serde_json::json!({
                                "pageCount":page_count,"textChars":text.chars().count(),"modelCalls":page_count+1,
                                "estimatedTokens":budget_checks.iter().map(|c|c["measurement"]["estimatedTokens"].clone()).collect::<Vec<_>>(),
                                "historyVisibleEveryCall":true,"excerptedUnderPressure":true,
                            })
                        );
                    }
                }
                "oversized-read-recovery" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(receipt["requestCounts"]["gate"], 4);
                    assert_eq!(receipt["requestCounts"]["model"], 4);
                    assert_eq!(tools.len(), 3);
                    assert_eq!(tools[0]["success"], false);
                    assert_eq!(
                        tools[0]["executionReceipt"]["failure"]["code"],
                        "AG_TOOL_RESULT_TOO_LARGE"
                    );
                    assert!(tools[0]["evidenceIds"].as_array().unwrap().is_empty());
                    assert_eq!(tools[1]["toolName"], "list_blocks");
                    assert_eq!(tools[1]["success"], true);
                    assert_eq!(tools[2]["toolName"], "read_doc");
                    assert_eq!(tools[2]["success"], true);
                    let report = events
                        .iter()
                        .find(|event| event["type"] == "project_report")
                        .expect("project report emitted after the completed turn");
                    let read_items = report["report"]["readItems"].as_array().unwrap();
                    assert!(read_items
                        .iter()
                        .any(|item| item["coverage"] == "structure_index"));
                    assert!(read_items
                        .iter()
                        .any(|item| item["coverage"] == "selected_blocks"));
                    assert!(!read_items
                        .iter()
                        .any(|item| item["coverage"] == "full_document"));
                    assert!(!receipt.to_string().contains("line 0799"));
                }
                "delay" => assert_eq!(receipt["status"], "cancelled"),
                "budget-initial" => {
                    assert_eq!(receipt["status"], "error");
                    assert!(tools.is_empty());
                    assert_eq!(budget_checks.len(), 1);
                    assert_eq!(budget_checks[0]["assemblyReason"], "initial");
                    assert_eq!(budget_checks[0]["exceeded"], true);
                    assert_eq!(receipt["requestCounts"]["gate"], 0);
                    assert_eq!(receipt["requestCounts"]["model"], 0);
                    assert!(events.iter().any(|event| event["type"] == "error"
                        && event["code"] == "AG_PROMPT_BUDGET_EXCEEDED"));
                }
                "budget-recovery" => {
                    assert_eq!(receipt["status"], "error");
                    assert_eq!(receipt["followUp"]["status"], "completed");
                    assert_eq!(receipt["followUp"]["sessionId"], receipt["runId"]);
                    assert_ne!(receipt["followUp"]["turnId"], receipt["runId"]);
                    assert!(tools.is_empty());
                    assert_eq!(budget_checks.len(), 2);
                    assert_eq!(budget_checks[0]["assemblyReason"], "initial");
                    assert_eq!(budget_checks[0]["exceeded"], true);
                    assert_eq!(budget_checks[1]["assemblyReason"], "initial");
                    assert_eq!(budget_checks[1]["exceeded"], false);
                    assert_eq!(budget_checks[1]["turnId"], receipt["followUp"]["turnId"]);
                    assert_eq!(receipt["requestCounts"]["gate"], 1);
                    assert_eq!(receipt["requestCounts"]["model"], 1);
                    assert!(events.iter().any(|event| event["type"] == "error"
                        && event["code"] == "AG_PROMPT_BUDGET_EXCEEDED"));
                    assert!(events.iter().any(|event| event["type"] == "done"
                        && event["turnId"] == receipt["followUp"]["turnId"]));
                }
                "budget-recovery-delay" => {
                    assert_eq!(receipt["status"], "error");
                    assert_eq!(receipt["followUp"]["status"], "cancelled");
                    assert_eq!(receipt["followUp"]["sessionId"], receipt["runId"]);
                    assert_ne!(receipt["followUp"]["turnId"], receipt["runId"]);
                    assert!(tools.is_empty());
                    assert_eq!(budget_checks.len(), 2);
                    assert_eq!(budget_checks[0]["exceeded"], true);
                    assert_eq!(budget_checks[1]["exceeded"], false);
                    assert_eq!(receipt["requestCounts"]["gate"], 1);
                    assert_eq!(receipt["requestCounts"]["model"], 1);
                    assert!(events.iter().any(|event| event["type"] == "error"
                        && event["code"] == "AG_PROMPT_BUDGET_EXCEEDED"));
                    assert!(events.iter().any(|event| event["type"] == "done"
                        && event["turnId"] == receipt["followUp"]["turnId"]
                        && event["stopReason"] == "cancelled"));
                }
                "budget-after-tool" => {
                    assert_eq!(receipt["status"], "error");
                    assert_eq!(tools.len(), 1);
                    assert_eq!(tools[0]["success"], true);
                    assert_eq!(budget_checks.len(), 2);
                    assert_eq!(budget_checks[0]["assemblyReason"], "initial");
                    assert_eq!(budget_checks[0]["exceeded"], false);
                    assert_eq!(budget_checks[1]["assemblyReason"], "tool_result");
                    assert_eq!(budget_checks[1]["exceeded"], true);
                    assert_eq!(budget_checks[1]["nextModelCallOrdinal"], 2);
                    assert_eq!(receipt["requestCounts"]["gate"], 1);
                    assert_eq!(receipt["requestCounts"]["model"], 1);
                    assert!(events.iter().any(|event| event["type"] == "error"
                        && event["code"] == "AG_PROMPT_BUDGET_EXCEEDED"));
                }
                "decision-chain" => {
                    assert_eq!(receipt["status"], "completed");
                    assert_eq!(tools.len(), 2);
                    assert!(tools
                        .iter()
                        .all(|tool| tool["success"] == true
                            && tool["toolName"] == "propose_decision"));
                    let trace = receipt["decisionTrace"].as_array().unwrap();
                    assert_eq!(trace.len(), 2);
                    assert_ne!(trace[0]["requestId"], trace[1]["requestId"]);
                    assert_eq!(trace[0]["taskRunId"], trace[1]["taskRunId"]);
                    assert!(trace.iter().all(|entry| entry["waitingStatus"] == "waiting"
                        && entry["outcome"] == "confirm"
                        && entry["alreadyResolved"] == false
                        && entry["duplicateAlreadyResolved"] == true));
                    let order: Vec<_> = events
                        .iter()
                        .filter_map(|event| match event["type"].as_str() {
                            Some("decision_request_proposed") => Some("proposed"),
                            Some("decision_request_resolved") => Some("resolved"),
                            Some("done") => Some("done"),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(
                        order,
                        ["proposed", "resolved", "proposed", "resolved", "done"]
                    );
                    assert_eq!(budget_checks.len(), 3);
                    assert_eq!(budget_checks[0]["assemblyReason"], "initial");
                    assert!(budget_checks[1..]
                        .iter()
                        .all(|check| check["assemblyReason"] == "decision_resume"
                            && check["exceeded"] == false));
                    assert_eq!(receipt["requestCounts"]["gate"], 3);
                    assert_eq!(receipt["requestCounts"]["model"], 3);
                }
                "invalid-json" | "truncated" => {
                    assert_eq!(receipt["status"], "error");
                    assert!(tools.is_empty());
                    let errors: Vec<_> = events
                        .iter()
                        .filter(|event| event["type"] == "error")
                        .collect();
                    assert_eq!(errors.len(), 1);
                    assert_eq!(
                        errors[0]["code"],
                        if scenario == "invalid-json" {
                            "AG_GW_STREAM_INVALID_JSON"
                        } else {
                            "AG_GW_STREAM_INCOMPLETE"
                        }
                    );
                    assert_eq!(errors[0]["diagnostic"]["observedAt"], "runtime");
                    assert_eq!(errors[0]["diagnostic"]["category"], "model_protocol");
                    assert_eq!(errors[0]["diagnostic"]["rootCauseStatus"], "unknown");
                }
                _ => {
                    assert_ne!(receipt["status"], "completed");
                    assert!(tools.is_empty());
                }
            }
        }
    }
}
