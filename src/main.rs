// mf — ManualForge: Law-of-Root living manuals for every repo.
// std-only. Sections are graph nodes; markdown/HTML/JSON/MCP are views.
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

// ============ data model ============
#[derive(Clone, Debug)]
struct Section {
    id: String,
    title: String,
    level: u8,
    body: String,
    parent: Option<String>,
    children: Vec<String>,
    xrefs: Vec<String>,
}

#[derive(Debug)]
struct Manual {
    title: String,
    repo: String,
    version: String,
    sections: Vec<Section>,
    index: HashMap<String, usize>,
}

impl Manual {
    fn get(&self, id: &str) -> Option<&Section> {
        self.index.get(id).map(|&i| &self.sections[i])
    }
}

// ============ parsing ============
fn parse_heading(line: &str) -> Option<(u8, String, String)> {
    let t = line.trim_start();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = t[hashes..].trim_start();
    let mut id_end = 0usize;
    for (i, c) in rest.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            id_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if id_end == 0 {
        return None;
    }
    let id = rest[..id_end].trim_end_matches('.').to_string();
    if id.is_empty() || !id.chars().next().unwrap().is_ascii_digit() {
        return None;
    }
    let title = rest[id_end..].trim().to_string();
    if title.is_empty() {
        return None;
    }
    let level = (id.matches('.').count() + 1) as u8;
    Some((level, id, title))
}

fn extract_xrefs(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let is_arrow = chars[i] == '→'
            || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1] == '>')
            || chars[i] == '§';
        if is_arrow {
            let mut j = if chars[i] == '-' { i + 2 } else { i + 1 };
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let mut k = j;
            while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '.') {
                k += 1;
            }
            if k > j {
                let id: String = chars[j..k].iter().collect();
                let id = id.trim_end_matches('.').to_string();
                if !id.is_empty() && !out.contains(&id) {
                    out.push(id);
                }
            }
            i = k.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

fn parse_manual(text: &str, title: &str, repo: &str) -> Manual {
    let mut sections: Vec<Section> = Vec::new();
    let mut cur: Option<(u8, String, String, Vec<String>)> = None;
    let mut flush = |cur: &mut Option<(u8, String, String, Vec<String>)>| {
        if let Some((_level, id, title, body_lines)) = cur.take() {
            let level = (id.matches('.').count() + 1) as u8;
            let body = body_lines.join("\n").trim().to_string();
            let xrefs = extract_xrefs(&body);
            let parent = id.rfind('.').map(|p| id[..p].to_string());
            sections.push(Section {
                id,
                title,
                level,
                body,
                parent,
                children: Vec::new(),
                xrefs,
            });
        }
    };
    for line in text.lines() {
        if let Some((_hlevel, id, title)) = parse_heading(line) {
            flush(&mut cur);
            cur = Some((0, id, title, Vec::new()));
        } else if let Some((_, _, _, body)) = cur.as_mut() {
            body.push(line.to_string());
        }
    }
    flush(&mut cur);
    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (i, s) in sections.iter().enumerate() {
        by_id.insert(s.id.clone(), i);
    }
    let child_pairs: Vec<(String, String)> = sections
        .iter()
        .filter_map(|s| s.parent.clone().map(|p| (p, s.id.clone())))
        .collect();
    for (p, c) in child_pairs {
        if let Some(&pi) = by_id.get(&p) {
            sections[pi].children.push(c);
        }
    }
    Manual {
        title: title.to_string(),
        repo: repo.to_string(),
        version: "0.1.0".to_string(),
        sections,
        index: by_id,
    }
}

// ============ json (hand-rolled) ============
fn jstr(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jarr(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(",")
    )
}

fn section_json(s: &Section) -> String {
    format!(
        "{{\"id\":{},\"title\":{},\"level\":{},\"body\":{},\"parent\":{},\"children\":{},\"xrefs\":{}}}",
        jstr(&s.id),
        jstr(&s.title),
        s.level,
        jstr(&s.body),
        s.parent.as_deref().map(jstr).unwrap_or_else(|| "null".to_string()),
        jarr(&s.children),
        jarr(&s.xrefs),
    )
}

fn manual_json(m: &Manual) -> String {
    let secs: Vec<String> = m.sections.iter().map(section_json).collect();
    format!(
        "{{\"title\":{},\"repo\":{},\"version\":{},\"sections\":[{}]}}",
        jstr(&m.title),
        jstr(&m.repo),
        jstr(&m.version),
        secs.join(",")
    )
}

fn graph_json(m: &Manual) -> String {
    let nodes: Vec<String> = m
        .sections
        .iter()
        .map(|s| {
            format!(
                "{{\"id\":{},\"title\":{},\"level\":{}}}",
                jstr(&s.id),
                jstr(&s.title),
                s.level
            )
        })
        .collect();
    let mut edges: Vec<String> = Vec::new();
    for s in &m.sections {
        if let Some(p) = &s.parent {
            edges.push(format!(
                "{{\"from\":{},\"to\":{},\"kind\":\"child\"}}",
                jstr(p),
                jstr(&s.id)
            ));
        }
        for x in &s.xrefs {
            edges.push(format!(
                "{{\"from\":{},\"to\":{},\"kind\":\"see\"}}",
                jstr(&s.id),
                jstr(x)
            ));
        }
    }
    format!(
        "{{\"nodes\":[{}],\"edges\":[{}]}}",
        nodes.join(","),
        edges.join(",")
    )
}

// ============ html view ============
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn render_body_html(body: &str) -> String {
    // paragraphs + xref links (§1.2 / → 1.2 become anchors)
    let mut html = String::new();
    for para in body.split("\n\n") {
        let p = para.trim();
        if p.is_empty() {
            continue;
        }
        // linkify xrefs
        let mut out = String::new();
        let chars: Vec<char> = p.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let is_arrow = chars[i] == '→'
                || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1] == '>')
                || chars[i] == '§';
            if is_arrow {
                let mut j = if chars[i] == '-' { i + 2 } else { i + 1 };
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let mut k = j;
                while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '.') {
                    k += 1;
                }
                if k > j {
                    let id: String = chars[j..k].iter().collect();
                    let id = id.trim_end_matches('.').to_string();
                    out.push_str(&format!("<a href=\"#{}\">→ {}</a>", id, html_escape(&id)));
                    i = k;
                    continue;
                }
            }
            match chars[i] {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '&' => out.push_str("&amp;"),
                c => out.push(c),
            }
            i += 1;
        }
        html.push_str(&format!("<p>{}</p>\n", out));
    }
    html
}

fn build_html(m: &Manual) -> String {
    let mut toc = String::new();
    for s in &m.sections {
        toc.push_str(&format!(
            "<div class=\"toc{i}\"><a href=\"#{}\"><b>{}</b> {}</a></div>\n",
            s.id,
            s.id,
            html_escape(&s.title),
            i = s.level
        ));
    }
    let mut body = String::new();
    for s in &m.sections {
        let tag = format!("h{}", (s.level + 1).min(6));
        body.push_str(&format!(
            "<section id=\"{}\"><{}><span class=\"addr\">{}</span> {}</{}>\n{}</section>\n",
            s.id,
            tag,
            html_escape(&s.id),
            html_escape(&s.title),
            tag,
            render_body_html(&s.body)
        ));
    }
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title><style>\
        body{{font-family:Georgia,serif;max-width:900px;margin:0 auto;padding:24px;color:#1a1a1a;line-height:1.55}}\
        .layout{{display:flex;gap:32px}}.toc{{position:sticky;top:24px;max-height:90vh;overflow:auto;min-width:220px;font-size:13px}}\
        .toc1{{margin-top:8px}}.toc2{{margin-left:12px}}.toc3{{margin-left:24px}}.toc4{{margin-left:36px}}\
        .toc a{{color:#333;text-decoration:none}}.toc a:hover{{text-decoration:underline}}\
        section{{margin:28px 0;padding-top:8px;border-top:1px solid #ddd}}\
        .addr{{color:#888;font-family:monospace}}.api{{font-size:12px;color:#888}}\
        a{{color:#0b5fff}}</style></head><body>\
        <h1>{}</h1><p class=\"api\">Living manual · {} · <a href=\"/api/sections\">API</a> · <a href=\"/api/graph\">graph</a></p>\
        <div class=\"layout\"><nav class=\"toc\">{}</nav><main>{}</main></div></body></html>",
        html_escape(&m.title),
        html_escape(&m.title),
        html_escape(&m.repo),
        toc,
        body
    )
}

// ============ repo scan (init) ============
fn read_head(path: &Path, max_lines: usize) -> String {
    fs::read_to_string(path)
        .map(|t| t.lines().take(max_lines).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

fn list_src_files(dir: &Path, out: &mut Vec<String>, depth: u8) {
    if depth > 3 {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "target" && name != "node_modules" {
                list_src_files(&p, out, depth + 1);
            }
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if ["rs", "py", "js", "ts", "go", "md", "toml", "json"].contains(&ext) {
                if let Ok(rel) = p.strip_prefix(dir) {
                    let _ = rel;
                }
                out.push(p.to_string_lossy().to_string());
            }
        }
    }
}

fn cmd_init(repo_dir: &str) {
    if let Some(name) = repo_dir.strip_prefix("external:") {
        let manual = format!(
            "# 1 Overview\n\n_(What is {name}? One paragraph. Link the official docs below.)_\n\n\
             Official documentation: _(paste URL)_\n\n\
             # 2 Why we use it\n\n_(What job does it do for us? What would break if it vanished?)_\n\n\
             # 3 How we use it\n\n_(Our configuration, the commands/flows we actually run.)_ See → 3.1.\n\n\
             ## 3.1 Common operations\n\n_(Fill in.)_\n\n\
             # 4 Compatibility notes\n\n_(Pinned version, known quirks, breaking changes we've hit.)_ See → 5.\n\n\
             ## 4.1 Version pin\n\n_(Fill in.)_\n\n\
             # 5 References\n\n- Official docs: _(URL)_\n- Our wrapper/adapter: _(repo path, → if internal)_\n"
        );
        fs::write("MANUAL.md", manual).expect("write MANUAL.md");
        println!("wrote MANUAL.md (external: {})", name);
        return;
    }
    let root = PathBuf::from(repo_dir);
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();
    let readme = ["README.md", "readme.md", "README"]
        .iter()
        .map(|f| root.join(f))
        .find(|p| p.exists())
        .map(|p| read_head(&p, 30))
        .unwrap_or_default();
    let mut files: Vec<String> = Vec::new();
    for sub in ["src", "."] {
        list_src_files(&root.join(sub), &mut files, 0);
    }
    files.sort();
    files.dedup();
    files.truncate(40);
    let file_list = files
        .iter()
        .map(|f| format!("- `{}`", f.replace(&format!("{}/", repo_dir), "")))
        .collect::<Vec<_>>()
        .join("\n");
    let deps = if root.join("Cargo.toml").exists() {
        let t = read_head(&root.join("Cargo.toml"), 40);
        t.lines()
            .skip_while(|l| !l.trim().starts_with("[dependencies]"))
            .take(15)
            .collect::<Vec<_>>()
            .join("\n")
    } else if root.join("package.json").exists() {
        read_head(&root.join("package.json"), 25)
    } else {
        "(no manifest found)".to_string()
    };
    let manual = format!(
        "# 1 Overview\n\n{readme}\n\nSee → 4 for the source layout and → 5 for dependencies.\n\n\
         ## 1.1 What it does\n\n_(Fill in: one paragraph on the problem this solves.)_\n\n\
         ## 1.2 Status\n\nLiving manual generated by `mf init`. Edit MANUAL.md, then `mf build`.\n\n\
         # 2 Installation\n\n_(Fill in: prerequisites and install steps.)_ See → 5 for dependencies.\n\n\
         ## 2.1 Prerequisites\n\n_(Fill in.)_\n\n\
         # 3 Usage\n\n_(Fill in: the three most common commands or flows.)_\n\n\
         ## 3.1 Quick start\n\n_(Fill in.)_\n\n\
         # 4 Architecture\n\nSource layout:\n\n{files}\n\n\
         ## 4.1 Key modules\n\n_(Fill in: what each major file/module owns.)_\n\n\
         # 5 Dependencies\n\n```\n{deps}\n```\n\n\
         # 6 Configuration\n\n_(Fill in: env vars, flags, config files.)_ See → 2.1 for prerequisites.\n\n\
         # 7 Troubleshooting\n\n_(Fill in: the errors people actually hit.)_\n\n\
         ## 7.1 Common errors\n\n_(Fill in.)_\n\n\
         # 8 Changelog\n\n## 8.1 Unreleased\n\n- Manual scaffolded by mf.\n"
    ,
        readme = if readme.is_empty() {
            "_(No README found — write the one-paragraph overview here.)_".to_string()
        } else {
            readme
        },
        files = if file_list.is_empty() {
            "_(no source files found)_".to_string()
        } else {
            file_list
        },
        deps = deps
    );
    let out = root.join("MANUAL.md");
    fs::write(&out, manual).expect("write MANUAL.md");
    println!("wrote {}", out.display());
}

// ============ build / check ============
fn load_manual() -> Manual {
    let text = fs::read_to_string("MANUAL.md").expect("MANUAL.md not found — run `mf init <repo>`");
    let title = text
        .lines()
        .find(|l| l.starts_with('#'))
        .and_then(|l| parse_heading(l))
        .map(|(_, _, t)| t)
        .unwrap_or_else(|| "Manual".to_string());
    let repo = env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    parse_manual(&text, &title, &repo)
}

fn cmd_build() {
    let m = load_manual();
    fs::create_dir_all("manual").ok();
    fs::write("manual/graph.json", graph_json(&m)).expect("write graph.json");
    fs::write("manual/sections.json", manual_json(&m)).expect("write sections.json");
    fs::write("manual/index.html", build_html(&m)).expect("write index.html");
    println!(
        "built manual/ ({} sections → graph.json, sections.json, index.html)",
        m.sections.len()
    );
}

fn cmd_check() {
    let m = load_manual();
    let mut errs: Vec<String> = Vec::new();
    let mut seen = HashMap::new();
    for s in &m.sections {
        if seen.insert(s.id.clone(), true).is_some() {
            errs.push(format!("duplicate address {}", s.id));
        }
        if let Some(p) = &s.parent {
            if m.get(p).is_none() {
                errs.push(format!("{}: parent {} missing", s.id, p));
            }
        }
        for x in &s.xrefs {
            if m.get(x).is_none() {
                errs.push(format!("{}: xref → {} does not resolve", s.id, x));
            }
        }
    }
    if errs.is_empty() {
        println!("law holds: {} sections, all addresses unique, all xrefs resolve", m.sections.len());
    } else {
        for e in errs {
            println!("VIOLATION: {}", e);
        }
        std::process::exit(1);
    }
}

// ============ tiny HTTP server ============
fn http_response(code: &str, ctype: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        code,
        ctype,
        body.len(),
        body
    )
}

fn url_decode(s: &str) -> String {
    let mut o = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                o.push(v as char);
                i += 3;
                continue;
            }
        }
        if b[i] == b'+' {
            o.push(' ');
        } else {
            o.push(b[i] as char);
        }
        i += 1;
    }
    o
}

fn cmd_serve(port: u16) {
    // reader mode: if index.json exists here, serve the multi-manual reader
    let reader_mode = Path::new("index.json").exists() && Path::new("reader.html").exists();
    let (reader_html, index_json) = if reader_mode {
        (
            fs::read_to_string("reader.html").unwrap_or_default(),
            fs::read_to_string("index.json").unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    };
    let m = if reader_mode {
        None
    } else {
        Some(load_manual())
    };
    // single-manual payloads (reader mode skips these)
    let (html, sections_json, graph, list_json) = match &m {
        Some(manual) => {
            let list: Vec<String> = manual
                .sections
                .iter()
                .map(|s| {
                    format!(
                        "{{\"id\":{},\"title\":{},\"level\":{}}}",
                        jstr(&s.id),
                        jstr(&s.title),
                        s.level
                    )
                })
                .collect();
            (
                build_html(manual),
                manual_json(manual),
                graph_json(manual),
                format!("[{}]", list.join(",")),
            )
        }
        None => (String::new(), String::new(), String::new(), String::new()),
    };
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind");
    if reader_mode {
        println!("mf serve (reader: all manuals) on http://127.0.0.1:{}/", port);
    } else {
        println!("mf serve on http://127.0.0.1:{}/", port);
    }
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            continue;
        }
        request.push_str(&line);
        loop {
            line.clear();
            if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
                break;
            }
        }
        let req_line = request.lines().next().unwrap_or("");
        let mut parts = req_line.split_whitespace();
        let _method = parts.next().unwrap_or("");
        let raw_path = parts.next().unwrap_or("/");
        let resp = if reader_mode {
            if raw_path == "/" || raw_path == "/reader.html" {
                http_response("200 OK", "text/html; charset=utf-8", &reader_html)
            } else if raw_path == "/api/index" {
                http_response("200 OK", "application/json", &index_json)
            } else {
                http_response("404 Not Found", "text/plain", "reader mode: / and /api/index only")
            }
        } else if raw_path == "/" || raw_path == "/index.html" {
            http_response("200 OK", "text/html; charset=utf-8", &html)
        } else if raw_path == "/api/sections" {
            http_response("200 OK", "application/json", &list_json)
        } else if raw_path == "/api/graph" {
            http_response("200 OK", "application/json", &graph)
        } else if raw_path == "/api/manual" {
            http_response("200 OK", "application/json", &sections_json)
        } else if let Some(id) = raw_path.strip_prefix("/api/section/") {
            let id = url_decode(id);
            match m.as_ref().and_then(|mm| mm.get(&id)) {
                Some(s) => http_response("200 OK", "application/json", &section_json(s)),
                None => http_response("404 Not Found", "application/json", "{\"error\":\"no such section\"}"),
            }
        } else if let Some(qs) = raw_path.strip_prefix("/api/search?q=") {
            let q = url_decode(qs).to_lowercase();
            let hits: Vec<String> = m
                .as_ref()
                .map(|mm| {
                    mm.sections
                        .iter()
                        .filter(|s| {
                            s.title.to_lowercase().contains(&q)
                                || s.body.to_lowercase().contains(&q)
                        })
                        .map(|s| {
                            let snip: String = s.body.chars().take(160).collect();
                            format!(
                                "{{\"id\":{},\"title\":{},\"snippet\":{}}}",
                                jstr(&s.id),
                                jstr(&s.title),
                                jstr(&snip)
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            http_response("200 OK", "application/json", &format!("[{}]", hits.join(",")))
        } else {
            http_response("404 Not Found", "text/plain", "no such route")
        };
        let _ = stream.write_all(resp.as_bytes());
    }
}

// ============ minimal JSON parser (for MCP) ============
#[derive(Debug, Clone)]
enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, k: &str) -> Option<&J> {
        if let J::Obj(pairs) = self {
            pairs.iter().find(|(kk, _)| kk == k).map(|(_, v)| v)
        } else {
            None
        }
    }
    fn as_str(&self) -> Option<&str> {
        if let J::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(s: &'a str) -> Self {
        P { b: s.as_bytes(), i: 0 }
    }
    fn ws(&mut self) {
        while self.i < self.b.len() && (self.b[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> u8 {
        *self.b.get(self.i).unwrap_or(&0)
    }
    fn parse(&mut self) -> Result<J, String> {
        self.ws();
        match self.peek() {
            b'n' => self.lit("null", J::Null),
            b't' => self.lit("true", J::Bool(true)),
            b'f' => self.lit("false", J::Bool(false)),
            b'"' => Ok(J::Str(self.string()?)),
            b'[' => self.array(),
            b'{' => self.object(),
            c if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err("bad json".to_string()),
        }
    }
    fn lit(&mut self, w: &str, v: J) -> Result<J, String> {
        if self.b[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            Err("bad literal".to_string())
        }
    }
    fn string(&mut self) -> Result<String, String> {
        // assumes opening quote at i
        self.i += 1;
        let mut o = String::new();
        while self.i < self.b.len() {
            let c = self.b[self.i];
            if c == b'"' {
                self.i += 1;
                return Ok(o);
            }
            if c == b'\\' {
                self.i += 1;
                if self.i >= self.b.len() {
                    break;
                }
                match self.b[self.i] {
                    b'"' => o.push('"'),
                    b'\\' => o.push('\\'),
                    b'n' => o.push('\n'),
                    b'r' => o.push('\r'),
                    b't' => o.push('\t'),
                    b'u' => {
                        if self.i + 4 < self.b.len() {
                            let hex = std::str::from_utf8(&self.b[self.i + 1..self.i + 5])
                                .map_err(|_| "bad unicode")?;
                            let cp = u32::from_str_radix(hex, 16).map_err(|_| "bad unicode")?;
                            o.push(char::from_u32(cp).unwrap_or('?'));
                            self.i += 4;
                        }
                    }
                    _ => {}
                }
                self.i += 1;
                continue;
            }
            o.push(c as char);
            self.i += 1;
        }
        Err("unterminated string".to_string())
    }
    fn array(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            if self.peek() == b']' {
                self.i += 1;
                return Ok(J::Arr(v));
            }
            v.push(self.parse()?);
            self.ws();
            match self.peek() {
                b',' => {
                    self.i += 1;
                }
                b']' => {
                    self.i += 1;
                    return Ok(J::Arr(v));
                }
                _ => return Err("bad array".to_string()),
            }
        }
    }
    fn object(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            if self.peek() == b'}' {
                self.i += 1;
                return Ok(J::Obj(v));
            }
            if self.peek() != b'"' {
                return Err("bad object key".to_string());
            }
            let k = self.string()?;
            self.ws();
            if self.peek() != b':' {
                return Err("bad object colon".to_string());
            }
            self.i += 1;
            let val = self.parse()?;
            v.push((k, val));
            self.ws();
            match self.peek() {
                b',' => {
                    self.i += 1;
                }
                b'}' => {
                    self.i += 1;
                    return Ok(J::Obj(v));
                }
                _ => return Err("bad object".to_string()),
            }
        }
    }
    fn number(&mut self) -> Result<J, String> {
        let start = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_digit()
                || b"+-eE.".contains(&self.b[self.i]))
        {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| "bad num")?;
        s.parse::<f64>().map(J::Num).map_err(|_| "bad num".to_string())
    }
}

fn parse_json(s: &str) -> Result<J, String> {
    let mut p = P::new(s);
    let v = p.parse()?;
    Ok(v)
}

// ============ MCP server (stdio, JSON-RPC) ============
fn mcp_result(id: &J, result_json: &str) -> String {
    let id_s = match id {
        J::Str(s) => jstr(s),
        J::Num(n) => format!("{}", n),
        _ => "null".to_string(),
    };
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}\n", id_s, result_json)
}

fn mcp_error(id: &J, code: i32, msg: &str) -> String {
    let id_s = match id {
        J::Str(s) => jstr(s),
        J::Num(n) => format!("{}", n),
        _ => "null".to_string(),
    };
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":{}}}}}\n",
        id_s,
        code,
        jstr(msg)
    )
}

fn cmd_mcp() {
    let m = load_manual();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    // support both bare JSON-RPC and LSP-style Content-Length framing
    let mut raw = String::new();
    stdin.lock().read_to_string(&mut raw).unwrap_or(0);
    // split LSP frames if present
    let mut msgs: Vec<String> = Vec::new();
    let mut rest = raw.as_str();
    loop {
        if let Some(hl) = rest.find("\r\n\r\n") {
            let head = &rest[..hl];
            let mut len: Option<usize> = None;
            for line in head.split("\r\n") {
                if let Some(v) = line.strip_prefix("Content-Length:") {
                    len = v.trim().parse().ok();
                }
            }
            if let Some(n) = len {
                let start = hl + 4;
                if rest.len() >= start + n {
                    msgs.push(rest[start..start + n].to_string());
                    rest = &rest[start + n..];
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }
    if msgs.is_empty() {
        // bare: one JSON object per line
        for line in raw.lines() {
            let t = line.trim();
            if !t.is_empty() {
                msgs.push(t.to_string());
            }
        }
    }
    for msg in msgs {
        let req = match parse_json(&msg) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let id = req.get("id").cloned().unwrap_or(J::Null);
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let resp = match method {
            "initialize" => mcp_result(
                &id,
                &format!(
                    "{{\"protocolVersion\":\"2024-11-05\",\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"mf-manual\",\"version\":\"0.1.0\"}}}}"
                ),
            ),
            "notifications/initialized" | "notifications/cancelled" => continue,
            "tools/list" => mcp_result(
                &id,
                "{\"tools\":[\
                {\"name\":\"get_section\",\"description\":\"Get one manual section by address (e.g. 2.3)\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"string\"}},\"required\":[\"id\"]}},\
                {\"name\":\"list_sections\",\"description\":\"List all sections with addresses and titles\",\"inputSchema\":{\"type\":\"object\",\"properties\":{}}},\
                {\"name\":\"search_sections\",\"description\":\"Search section titles and bodies\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"query\":{\"type\":\"string\"}},\"required\":[\"query\"]}},\
                {\"name\":\"get_children\",\"description\":\"Get child sections of an address\",\"inputSchema\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"string\"}},\"required\":[\"id\"]}}\
                ]}",
            ),
            "tools/call" => {
                let params = req.get("params");
                let name = params.and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                let args = params.and_then(|p| p.get("arguments"));
                let text = match name {
                    "get_section" => {
                        let sid = args.and_then(|a| a.get("id")).and_then(|v| v.as_str()).unwrap_or("");
                        match m.get(sid) {
                            Some(s) => section_json(s),
                            None => format!("{{\"error\":{}}}", jstr(&format!("no section {}", sid))),
                        }
                    }
                    "list_sections" => {
                        let items: Vec<String> = m.sections.iter().map(|s| {
                            format!("{{\"id\":{},\"title\":{},\"level\":{}}}", jstr(&s.id), jstr(&s.title), s.level)
                        }).collect();
                        format!("[{}]", items.join(","))
                    }
                    "search_sections" => {
                        let q = args.and_then(|a| a.get("query")).and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
                        let hits: Vec<String> = m.sections.iter()
                            .filter(|s| s.title.to_lowercase().contains(&q) || s.body.to_lowercase().contains(&q))
                            .map(|s| {
                                let snip: String = s.body.chars().take(160).collect();
                                format!("{{\"id\":{},\"title\":{},\"snippet\":{}}}", jstr(&s.id), jstr(&s.title), jstr(&snip))
                            }).collect();
                        format!("[{}]", hits.join(","))
                    }
                    "get_children" => {
                        let sid = args.and_then(|a| a.get("id")).and_then(|v| v.as_str()).unwrap_or("");
                        match m.get(sid) {
                            Some(s) => {
                                let kids: Vec<String> = s.children.iter().filter_map(|c| m.get(c)).map(section_json).collect();
                                format!("[{}]", kids.join(","))
                            }
                            None => format!("{{\"error\":{}}}", jstr(&format!("no section {}", sid))),
                        }
                    }
                    _ => format!("{{\"error\":{}}}", jstr("unknown tool")),
                };
                // wrap as MCP content
                let content = format!("{{\"type\":\"text\",\"text\":{}}}", jstr(&text));
                mcp_result(&id, &format!("{{\"content\":[{}]}}", content))
            }
            _ => mcp_error(&id, -32601, "method not found"),
        };
        let _ = out.write_all(resp.as_bytes());
        let _ = out.flush();
    }
}

// ============ multi-manual index + reader ============
// `mf index <repos-root>` scans every <repo>/manual/sections.json beneath
// root and writes index.json + reader.html: one reader over ALL manuals.
fn cmd_index(root: &str) {
    let rootp = PathBuf::from(root);
    let mut manuals: Vec<String> = Vec::new();
    let mut all_sections: Vec<String> = Vec::new();
    let entries = fs::read_dir(&rootp).unwrap_or_else(|_| panic!("cannot read {}", root));
    let mut repos: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    repos.sort();
    for repo_dir in repos {
        // a manual counts if it has MANUAL.md (build output optional)
        let mm = repo_dir.join("MANUAL.md");
        if !mm.exists() {
            continue;
        }
        let mtext = fs::read_to_string(&mm).unwrap_or_default();
        let repo_name = repo_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo")
            .to_string();
        let title = mtext
            .lines()
            .find(|l| l.starts_with('#'))
            .and_then(|l| parse_heading(l))
            .map(|(_, _, t)| t)
            .unwrap_or_else(|| repo_name.clone());
        let m = parse_manual(&mtext, &title, &repo_name);
        manuals.push(format!(
            "{{\"repo\":{},\"title\":{},\"count\":{}}}",
            jstr(&repo_name),
            jstr(&m.title),
            m.sections.len()
        ));
        for s in &m.sections {
            all_sections.push(format!(
                "{{\"repo\":{},\"id\":{},\"title\":{},\"level\":{},\"body\":{}}}",
                jstr(&repo_name),
                jstr(&s.id),
                jstr(&s.title),
                s.level,
                jstr(&s.body)
            ));
        }
    }
    let index_json = format!(
        "{{\"manuals\":[{}],\"sections\":[{}]}}",
        manuals.join(","),
        all_sections.join(",")
    );
    fs::write("index.json", &index_json).expect("write index.json");
    fs::write("reader.html", reader_html()).expect("write reader.html");
    println!(
        "indexed {} manuals, {} sections → index.json + reader.html",
        manuals.len(),
        all_sections.len()
    );
}

fn reader_html() -> String {
    r##"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Manuals — Reader</title><style>
body{font-family:Georgia,serif;max-width:980px;margin:0 auto;padding:24px;color:#1a1a1a;line-height:1.55}
header{display:flex;gap:12px;align-items:center;flex-wrap:wrap}
#q{flex:1;min-width:200px;padding:8px;font-size:15px}
#manuals{display:flex;gap:8px;flex-wrap:wrap;margin:12px 0}
.chip{border:1px solid #999;border-radius:14px;padding:4px 12px;font-size:13px;cursor:pointer;background:#fff}
.chip.on{background:#1a1a1a;color:#fff}
#list{margin-top:16px}.hit{border-top:1px solid #ddd;padding:10px 0;cursor:pointer}
.hit:hover{background:#f7f7f7}.addr{font-family:monospace;color:#888;font-size:12px}
.repo{font-size:11px;text-transform:uppercase;letter-spacing:1px;color:#0b5fff}
#view{margin-top:16px}.back{font-size:13px;cursor:pointer;color:#0b5fff}
pre{background:#f4f4f4;padding:12px;overflow:auto}
a{color:#0b5fff}</style></head><body>
<header><h1 style="margin:0">Manuals</h1><input id="q" placeholder="search every manual…"></header>
<div id="manuals"></div><div id="list"></div><div id="view" style="display:none"></div>
<script>
let IDX=null, activeRepo=null;
async function load(){IDX=await (await fetch('/api/index')).json();renderChips();renderList('');}
function renderChips(){
 const el=document.getElementById('manuals');el.innerHTML='';
 const all=document.createElement('span');all.className='chip'+(!activeRepo?' on':'');all.textContent='all';
 all.onclick=()=>{activeRepo=null;renderChips();renderList(document.getElementById('q').value);};el.appendChild(all);
 IDX.manuals.forEach(m=>{const c=document.createElement('span');c.className='chip'+(activeRepo===m.repo?' on':'');
  c.textContent=m.repo+' ('+m.count+')';c.onclick=()=>{activeRepo=m.repo;renderChips();renderList(document.getElementById('q').value);};el.appendChild(c);});
}
function secs(){return IDX.sections.filter(s=>!activeRepo||s.repo===activeRepo);}
function renderList(q){
 q=q.toLowerCase();const el=document.getElementById('list');document.getElementById('view').style.display='none';el.style.display='';
 const hits=secs().filter(s=>!q||s.title.toLowerCase().includes(q)||s.body.toLowerCase().includes(q));
 el.innerHTML=hits.slice(0,300).map((s,i)=>'<div class="hit" data-i="'+IDX.sections.indexOf(s)+'"><span class="repo">'+esc(s.repo)+'</span> <span class="addr">'+esc(s.id)+'</span><br><b>'+esc(s.title)+'</b></div>').join('')
  ||'<p>No matches.</p>';
 el.querySelectorAll('.hit').forEach(h=>h.onclick=()=>show(+h.dataset.i));
}
function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
function show(i){
 const s=IDX.sections[i];const el=document.getElementById('view');
 let body=esc(s.body).replace(/&gt; (\d[\d.]*)/g,'<a href="#" data-x="$1">&gt; $1</a>');
 el.innerHTML='<div class="back" onclick="back()">← back</div><span class="repo">'+esc(s.repo)+'</span> <span class="addr">'+esc(s.id)+'</span><h2>'+esc(s.title)+'</h2><div>'+body.split('\n\n').map(p=>'<p>'+p.replace(/\n/g,'<br>')+'</p>').join('')+'</div>';
 el.querySelectorAll('[data-x]').forEach(a=>a.onclick=e=>{e.preventDefault();const t=IDX.sections.findIndex(x=>x.repo===s.repo&&x.id===a.dataset.x);if(t>=0)show(t);});
 document.getElementById('list').style.display='none';el.style.display='';window.scrollTo(0,0);
}
function back(){document.getElementById('view').style.display='none';document.getElementById('list').style.display='';}
document.getElementById('q').addEventListener('input',e=>renderList(e.target.value));
load();</script></body></html>"##.to_string()
}

// ============ main ============
fn usage() {
    eprintln!("mf — ManualForge: Law-of-Root living manuals");
    eprintln!("  mf init <repo-dir>   scaffold MANUAL.md from a repo");
    eprintln!("  mf init external:<name>  scaffold MANUAL.md for third-party tooling");
    eprintln!("  mf build             MANUAL.md → manual/graph.json + sections.json + index.html");
    eprintln!("  mf check             verify the law (unique addresses, resolved xrefs)");
    eprintln!("  mf index <repos-root> index every manual below root → index.json + reader.html");
    eprintln!("  mf serve [port]      HTTP API + human view (default 8471; serves reader if index.json present)");
    eprintln!("  mf mcp               MCP server on stdio");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
        return;
    }
    match args[1].as_str() {
        "init" => {
            if args.len() < 3 {
                eprintln!("usage: mf init <repo-dir>");
                return;
            }
            cmd_init(&args[2]);
        }
        "build" => cmd_build(),
        "check" => cmd_check(),
        "index" => {
            if args.len() < 3 {
                eprintln!("usage: mf index <repos-root>");
                return;
            }
            cmd_index(&args[2]);
        }
        "serve" => {
            let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(8471);
            cmd_serve(port);
        }
        "mcp" => cmd_mcp(),
        _ => usage(),
    }
}
