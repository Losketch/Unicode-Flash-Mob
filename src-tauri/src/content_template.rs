use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentTemplate {
    Text {
        value: String,
    },
    External {
        executable: PathBuf,
        #[serde(default)]
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct TemplateContext<'a> {
    pub character: &'a str,
    pub glyph_expression: &'a str,
    pub description: &'a str,
}

/// Resolves built-in and user-defined content placeholders.
///
/// External templates are executed directly (without a shell) and cached by
/// their fully resolved command line so frame rendering never respawns the same
/// command for identical inputs.
pub struct ContentTemplateResolver {
    definitions: BTreeMap<String, ContentTemplate>,
    external_cache: Mutex<HashMap<String, String>>,
}

impl ContentTemplateResolver {
    pub fn new(definitions: BTreeMap<String, ContentTemplate>) -> Self {
        Self {
            definitions,
            external_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve(&self, template: &str, context: &TemplateContext<'_>) -> Result<String> {
        let mut stack = HashSet::new();
        self.resolve_inner(template, context, &mut stack)
    }

    fn resolve_inner(
        &self,
        template: &str,
        context: &TemplateContext<'_>,
        stack: &mut HashSet<String>,
    ) -> Result<String> {
        let mut output = String::with_capacity(template.len());
        let mut cursor = 0usize;

        while let Some(relative_start) = template[cursor..].find('{') {
            let start = cursor + relative_start;
            output.push_str(&template[cursor..start]);
            let Some(relative_end) = template[start + 1..].find('}') else {
                output.push_str(&template[start..]);
                return Ok(output);
            };
            let end = start + 1 + relative_end;
            let key = template[start + 1..end].trim();

            if let Some(value) = self.resolve_placeholder(key, context, stack)? {
                output.push_str(&value);
            } else {
                output.push_str(&template[start..=end]);
            }
            cursor = end + 1;
        }

        output.push_str(&template[cursor..]);
        Ok(output)
    }

    fn resolve_placeholder(
        &self,
        key: &str,
        context: &TemplateContext<'_>,
        stack: &mut HashSet<String>,
    ) -> Result<Option<String>> {
        let built_in = match key {
            "char" => Some(context.character.to_string()),
            "glyph" | "code" => Some(context.glyph_expression.to_string()),
            "description" => Some(context.description.to_string()),
            _ => None,
        };
        if built_in.is_some() {
            return Ok(built_in);
        }

        let Some(definition) = self.definitions.get(key) else {
            return Ok(None);
        };
        if !stack.insert(key.to_string()) {
            anyhow::bail!("Content template cycle detected at {{{key}}}");
        }

        let resolved = match definition {
            ContentTemplate::Text { value } => self.resolve_inner(value, context, stack),
            ContentTemplate::External { executable, args } => {
                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_inner(arg, context, stack))
                    .collect::<Result<Vec<_>>>()?;
                self.run_external(executable, &resolved_args)
            }
        };
        stack.remove(key);
        resolved.map(Some)
    }

    fn run_external(&self, executable: &Path, args: &[String]) -> Result<String> {
        let mut cache_key = executable.to_string_lossy().into_owned();
        for arg in args {
            cache_key.push('\0');
            cache_key.push_str(arg);
        }

        if let Some(cached) = self
            .external_cache
            .lock()
            .map_err(|_| anyhow::anyhow!("Content template cache lock poisoned"))?
            .get(&cache_key)
            .cloned()
        {
            return Ok(cached);
        }

        let output = Command::new(executable)
            .args(args)
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute content template program: {}",
                    executable.display()
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Content template program {} exited with {:?}: {}",
                executable.display(),
                output.status.code(),
                stderr.trim()
            );
        }

        let value = String::from_utf8(output.stdout)
            .context("Content template program returned non-UTF-8 output")?
            .trim_end_matches(&['\r', '\n'][..])
            .to_string();
        self.external_cache
            .lock()
            .map_err(|_| anyhow::anyhow!("Content template cache lock poisoned"))?
            .insert(cache_key, value.clone());
        Ok(value)
    }
}

#[cfg(test)]
#[path = "../tests/unit/content_template.rs"]
mod tests;
