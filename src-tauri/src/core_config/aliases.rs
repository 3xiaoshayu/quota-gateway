use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OAuthAliasChannel {
    pub(crate) key: &'static str,
    pub(crate) provider: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) protocol: &'static str,
    pub(crate) supports_reasoning: bool,
    pub(crate) supports_fast: bool,
    pub(crate) force_mapping: bool,
}

pub(crate) const OAUTH_ALIAS_CHANNELS: [OAuthAliasChannel; 7] = [
    OAuthAliasChannel {
        key: "vertex",
        provider: "Vertex OAuth",
        kind: "vertex-oauth",
        protocol: "gemini",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: false,
    },
    OAuthAliasChannel {
        key: "aistudio",
        provider: "AI Studio OAuth",
        kind: "aistudio-oauth",
        protocol: "gemini",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: false,
    },
    OAuthAliasChannel {
        key: "antigravity",
        provider: "Antigravity OAuth",
        kind: "antigravity-oauth",
        protocol: "antigravity",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: true,
    },
    OAuthAliasChannel {
        key: "claude",
        provider: "Claude OAuth",
        kind: "claude-oauth",
        protocol: "claude",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: false,
    },
    OAuthAliasChannel {
        key: "codex",
        provider: "Codex OAuth",
        kind: "codex-oauth",
        protocol: "codex",
        supports_reasoning: true,
        supports_fast: true,
        force_mapping: false,
    },
    OAuthAliasChannel {
        key: "kimi",
        provider: "Kimi OAuth",
        kind: "kimi-oauth",
        protocol: "openai",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: false,
    },
    OAuthAliasChannel {
        key: "xai",
        provider: "xAI OAuth",
        kind: "xai-oauth",
        protocol: "codex",
        supports_reasoning: true,
        supports_fast: false,
        force_mapping: false,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OAuthModelDefinitions {
    pub(crate) channel: OAuthAliasChannel,
    pub(crate) models: Vec<CodexModelDefinition>,
}

pub(crate) fn oauth_alias_channel(channel: &str) -> Option<OAuthAliasChannel> {
    OAUTH_ALIAS_CHANNELS
        .iter()
        .copied()
        .find(|candidate| candidate.key.eq_ignore_ascii_case(channel))
}

pub(crate) fn normalize_oauth_alias_channel(value: &str) -> Option<&'static str> {
    let value = value.trim().to_ascii_lowercase().replace('_', "-");
    match value.as_str() {
        "vertex" | "vertex-ai" => Some("vertex"),
        "aistudio" | "ai-studio" | "gemini" | "gemini-cli" => Some("aistudio"),
        "antigravity" | "anti-gravity" => Some("antigravity"),
        "claude" | "anthropic" => Some("claude"),
        "codex" => Some("codex"),
        "kimi" | "moonshot" => Some("kimi"),
        "xai" | "x-ai" | "grok" => Some("xai"),
        _ => None,
    }
}

pub(crate) async fn fetch_active_oauth_alias_channels(
    config: &GuiConfigFile,
) -> Result<std::collections::HashSet<String>, String> {
    let client = management_http_client()?;
    let response = client
        .get(management_endpoint(config, "auth-files")?)
        .header("Authorization", management_authorization(config)?)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| format_management_request_error("读取 OAuth 凭据来源失败", &error))?;
    let payload = read_management_value(response).await?;
    let files = payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .or_else(|| payload.as_array())
        .ok_or_else(|| "OAuth 凭据来源响应缺少 files 数组".to_string())?;
    Ok(files
        .iter()
        .filter(|file| {
            !file
                .get("disabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                && !file
                    .get("unavailable")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
        })
        .filter_map(|file| {
            ["provider", "type"]
                .into_iter()
                .find_map(|key| {
                    file.get(key)
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                })
                .and_then(normalize_oauth_alias_channel)
        })
        .map(str::to_string)
        .collect())
}

pub(crate) async fn fetch_management_config_yaml(config: &GuiConfigFile) -> Result<String, String> {
    let client = management_http_client()?;
    let response = client
        .get(management_endpoint(config, "config.yaml")?)
        .header("Authorization", management_authorization(config)?)
        .header(
            reqwest::header::ACCEPT,
            "application/yaml,text/yaml,text/plain",
        )
        .send()
        .await
        .map_err(|error| format_management_request_error("读取内核 YAML 配置失败", &error))?;
    read_management_text(response).await
}

pub(crate) async fn put_management_config_yaml(
    config: &GuiConfigFile,
    content: &str,
) -> Result<(), String> {
    let client = management_http_client()?;
    let response = client
        .put(management_endpoint(config, "config.yaml")?)
        .header("Authorization", management_authorization(config)?)
        .header(reqwest::header::CONTENT_TYPE, "application/yaml")
        .body(content.to_string())
        .send()
        .await
        .map_err(|error| format_management_request_error("保存内核 YAML 配置失败", &error))?;
    read_management_value(response).await.map(|_| ())
}

pub(crate) async fn put_management_oauth_model_aliases(
    config: &GuiConfigFile,
    aliases: &serde_json::Value,
) -> Result<(), String> {
    let client = management_http_client()?;
    let response = client
        .put(management_endpoint(config, "oauth-model-alias")?)
        .header("Authorization", management_authorization(config)?)
        .json(aliases)
        .send()
        .await
        .map_err(|error| format_management_request_error("保存 OAuth 模型别名失败", &error))?;
    read_management_value(response).await.map(|_| ())
}

#[derive(Debug, PartialEq)]
pub(crate) struct ManagementAliasConfigChanges {
    pub(crate) oauth_model_aliases: Option<serde_json::Value>,
    pub(crate) update_config_yaml: bool,
}

pub(crate) fn management_alias_config_changes(
    current: &str,
    updated: &str,
) -> Result<ManagementAliasConfigChanges, String> {
    let split = |content: &str| {
        let mut document = serde_norway::from_str::<serde_norway::Value>(content)
            .map_err(|error| format!("解析内核 YAML 配置失败: {error}"))?;
        let root = document
            .as_mapping_mut()
            .ok_or_else(|| "内核配置顶层必须是 YAML 映射".to_string())?;
        let oauth_aliases = root
            .remove(yaml_key("oauth-model-alias"))
            .unwrap_or_else(|| serde_norway::Value::Mapping(serde_norway::Mapping::new()));
        Ok::<_, String>((document, oauth_aliases))
    };
    let (current_without_oauth, current_oauth) = split(current)?;
    let (updated_without_oauth, updated_oauth) = split(updated)?;
    let oauth_model_aliases = if current_oauth == updated_oauth {
        None
    } else {
        Some(
            serde_json::to_value(updated_oauth)
                .map_err(|error| format!("序列化 OAuth 模型别名失败: {error}"))?,
        )
    };
    Ok(ManagementAliasConfigChanges {
        oauth_model_aliases,
        update_config_yaml: current_without_oauth != updated_without_oauth,
    })
}

pub(crate) async fn put_management_alias_config_changes(
    config: &GuiConfigFile,
    current: &str,
    updated: &str,
) -> Result<(), String> {
    if updated == current {
        return Ok(());
    }
    let changes = management_alias_config_changes(current, updated)?;
    if let Some(oauth_model_aliases) = changes.oauth_model_aliases.as_ref() {
        // The dedicated endpoint refreshes CPA's OAuth model registry immediately.
        // Writing the same section only through config.yaml updates the file and
        // management snapshot, but can leave /v1/models and routing stale.
        put_management_oauth_model_aliases(config, oauth_model_aliases).await?;
    }
    if changes.update_config_yaml {
        put_management_config_yaml(config, updated).await?;
    }
    Ok(())
}

pub(crate) async fn ensure_claude_desktop_model_aliases(
    config: &GuiConfigFile,
    mappings: &ClaudeDesktopModelMappings,
    models: &[AgentModelOption],
) -> Result<(), String> {
    let content = fetch_management_config_yaml(config).await?;
    let updated = match ensure_claude_desktop_model_aliases_in_yaml(&content, mappings, models) {
        Ok(updated) => updated,
        Err(_) => {
            let definitions = fetch_oauth_model_definitions(config).await;
            ensure_claude_desktop_model_aliases_with_oauth_definitions_in_yaml(
                &content,
                mappings,
                models,
                &definitions,
            )?
        }
    };
    put_management_alias_config_changes(config, &content, &updated).await
}

pub(crate) fn ensure_claude_desktop_model_aliases_in_yaml(
    content: &str,
    mappings: &ClaudeDesktopModelMappings,
    models: &[AgentModelOption],
) -> Result<String, String> {
    ensure_claude_desktop_model_aliases_with_codex_oauth_in_yaml(content, mappings, models, &[])
}

pub(crate) fn ensure_claude_desktop_model_aliases_with_codex_oauth_in_yaml(
    content: &str,
    mappings: &ClaudeDesktopModelMappings,
    models: &[AgentModelOption],
    codex_oauth_models: &[CodexModelDefinition],
) -> Result<String, String> {
    let codex_channel = oauth_alias_channel("codex")
        .ok_or_else(|| "缺少 Codex OAuth 别名 channel 定义".to_string())?;
    let definitions = [OAuthModelDefinitions {
        channel: codex_channel,
        models: codex_oauth_models.to_vec(),
    }];
    ensure_claude_desktop_model_aliases_with_oauth_definitions_in_yaml(
        content,
        mappings,
        models,
        &definitions,
    )
}

pub(crate) fn ensure_claude_desktop_model_aliases_with_oauth_definitions_in_yaml(
    content: &str,
    mappings: &ClaudeDesktopModelMappings,
    models: &[AgentModelOption],
    oauth_model_definitions: &[OAuthModelDefinitions],
) -> Result<String, String> {
    let mut document = yaml_serde_edit::YamlValue::parse(content)
        .map_err(|error| format!("解析内核 YAML 配置失败: {error}"))?;
    let mut updated = document.get().clone();
    let root = updated
        .as_mapping_mut()
        .ok_or_else(|| "内核配置顶层必须是 YAML 映射".to_string())?;

    for (alias, source_model) in [
        (CLAUDE_DESKTOP_OPUS_MODEL_ID, mappings.opus.as_str()),
        (CLAUDE_DESKTOP_SONNET_MODEL_ID, mappings.sonnet.as_str()),
        (CLAUDE_DESKTOP_HAIKU_MODEL_ID, mappings.haiku.as_str()),
    ] {
        let direct_alias = models
            .iter()
            .any(|model| model.name.eq_ignore_ascii_case(source_model) && model.is_alias);
        if direct_alias {
            if !source_model.eq_ignore_ascii_case(alias) {
                remove_managed_claude_model_alias(root, alias)?;
            }
        } else {
            ensure_claude_desktop_model_alias(root, source_model, alias, oauth_model_definitions)?;
        }
    }
    render_updated_core_yaml(&mut document, updated)
}

#[cfg(test)]
pub(crate) fn remove_managed_claude_model_aliases_in_yaml(content: &str) -> Result<String, String> {
    let mut document = yaml_serde_edit::YamlValue::parse(content)
        .map_err(|error| format!("解析内核 YAML 配置失败: {error}"))?;
    let mut updated = document.get().clone();
    let root = updated
        .as_mapping_mut()
        .ok_or_else(|| "内核配置顶层必须是 YAML 映射".to_string())?;
    let mut changed = false;
    for alias in [
        CLAUDE_DESKTOP_OPUS_MODEL_ID,
        CLAUDE_DESKTOP_SONNET_MODEL_ID,
        CLAUDE_DESKTOP_HAIKU_MODEL_ID,
    ] {
        changed |= remove_managed_claude_model_alias(root, alias)?;
    }
    if !changed {
        return Ok(content.to_string());
    }
    render_updated_core_yaml(&mut document, updated)
}

pub(crate) fn remove_managed_claude_model_alias(
    root: &mut serde_norway::Mapping,
    alias: &str,
) -> Result<bool, String> {
    let mut changed = false;
    for section in MODEL_ALIAS_CONFIG_SECTIONS {
        let Some(providers) = yaml_mapping_value_mut(root, section) else {
            continue;
        };
        let providers = providers
            .as_sequence_mut()
            .ok_or_else(|| format!("{section} 必须是数组"))?;
        for provider in providers {
            let Some(provider) = provider.as_mapping_mut() else {
                continue;
            };
            let Some(models) = yaml_mapping_value_mut(provider, "models") else {
                continue;
            };
            let models = models
                .as_sequence_mut()
                .ok_or_else(|| format!("{section}.models 必须是数组"))?;
            let before = models.len();
            models.retain(|model| !is_managed_claude_model_alias(model, alias));
            changed |= models.len() != before;
        }
    }
    changed |= remove_oauth_claude_model_alias(root, alias, true)?;
    Ok(changed)
}

pub(crate) fn remove_existing_claude_model_alias(
    root: &mut serde_norway::Mapping,
    alias: &str,
) -> Result<bool, String> {
    let mut changed = false;
    for section in MODEL_ALIAS_CONFIG_SECTIONS {
        let Some(providers) = yaml_mapping_value_mut(root, section) else {
            continue;
        };
        let providers = providers
            .as_sequence_mut()
            .ok_or_else(|| format!("{section} 必须是数组"))?;
        for provider in providers {
            let Some(provider) = provider.as_mapping_mut() else {
                continue;
            };
            let Some(models) = yaml_mapping_value_mut(provider, "models") else {
                continue;
            };
            let models = models
                .as_sequence_mut()
                .ok_or_else(|| format!("{section}.models 必须是数组"))?;
            let before = models.len();
            models.retain(|model| {
                configured_model_identity(model)
                    .is_none_or(|(_, client_model, _)| !client_model.eq_ignore_ascii_case(alias))
            });
            changed |= models.len() != before;
        }
    }
    changed |= remove_oauth_claude_model_alias(root, alias, false)?;
    Ok(changed)
}

pub(crate) fn remove_oauth_claude_model_alias(
    root: &mut serde_norway::Mapping,
    alias: &str,
    managed_only: bool,
) -> Result<bool, String> {
    let Some(oauth_aliases) = yaml_mapping_value_mut(root, "oauth-model-alias") else {
        return Ok(false);
    };
    let oauth_aliases = oauth_aliases
        .as_mapping_mut()
        .ok_or_else(|| "oauth-model-alias 必须是 YAML 映射".to_string())?;
    let mut changed = false;
    let mut empty_channels = Vec::new();
    for (channel, entries) in oauth_aliases.iter_mut() {
        let channel_name = channel.as_str().unwrap_or("unknown");
        let entries = entries
            .as_sequence_mut()
            .ok_or_else(|| format!("oauth-model-alias.{channel_name} 必须是数组"))?;
        let before = entries.len();
        entries.retain(|entry| {
            if managed_only {
                !is_managed_claude_model_alias(entry, alias)
            } else {
                configured_model_identity(entry)
                    .is_none_or(|(_, client_model, _)| !client_model.eq_ignore_ascii_case(alias))
            }
        });
        if entries.len() != before {
            changed = true;
            if entries.is_empty() {
                empty_channels.push(channel.clone());
            }
        }
    }
    for channel in empty_channels {
        oauth_aliases.remove(&channel);
    }
    let remove_oauth_section = changed && oauth_aliases.is_empty();
    if remove_oauth_section {
        root.remove(yaml_key("oauth-model-alias"));
    }
    Ok(changed)
}

pub(crate) fn managed_claude_alias_display_name(alias: &str) -> Option<&'static str> {
    if alias.eq_ignore_ascii_case(CLAUDE_DESKTOP_OPUS_MODEL_ID) {
        Some(MANAGED_CLAUDE_OPUS_ALIAS_DISPLAY_NAME)
    } else if alias.eq_ignore_ascii_case(CLAUDE_DESKTOP_SONNET_MODEL_ID) {
        Some(MANAGED_CLAUDE_SONNET_ALIAS_DISPLAY_NAME)
    } else if alias.eq_ignore_ascii_case(CLAUDE_DESKTOP_HAIKU_MODEL_ID) {
        Some(MANAGED_CLAUDE_HAIKU_ALIAS_DISPLAY_NAME)
    } else {
        None
    }
}

pub(crate) fn is_managed_claude_model_alias(model: &serde_norway::Value, alias: &str) -> bool {
    let Some(expected_display_name) = managed_claude_alias_display_name(alias) else {
        return false;
    };
    let Some((_, client_model, display_name)) = configured_model_identity(model) else {
        return false;
    };
    client_model.eq_ignore_ascii_case(alias)
        && display_name
            .as_deref()
            .is_some_and(|value| value == expected_display_name)
}

pub(crate) fn configured_managed_claude_alias_exists(
    root: &serde_norway::Mapping,
    alias: &str,
) -> bool {
    let configured_provider_alias = MODEL_ALIAS_CONFIG_SECTIONS.iter().any(|section| {
        yaml_mapping_value(root, section)
            .and_then(serde_norway::Value::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(serde_norway::Value::as_mapping)
            .filter_map(|provider| yaml_mapping_value(provider, "models"))
            .filter_map(serde_norway::Value::as_sequence)
            .flatten()
            .any(|model| is_managed_claude_model_alias(model, alias))
    });
    configured_provider_alias
        || yaml_mapping_value(root, "oauth-model-alias")
            .and_then(serde_norway::Value::as_mapping)
            .into_iter()
            .flat_map(|channels| channels.values())
            .filter_map(serde_norway::Value::as_sequence)
            .flatten()
            .any(|model| is_managed_claude_model_alias(model, alias))
}

pub(crate) fn ensure_claude_desktop_model_alias(
    root: &mut serde_norway::Mapping,
    source_model: &str,
    alias: &str,
    oauth_model_definitions: &[OAuthModelDefinitions],
) -> Result<(), String> {
    if let Some((existing_source, _)) = configured_model_client_identity(root, alias) {
        if existing_source.eq_ignore_ascii_case(source_model)
            && configured_managed_claude_alias_exists(root, alias)
        {
            return Ok(());
        }
        remove_existing_claude_model_alias(root, alias)?;
    }
    if append_claude_desktop_model_alias(root, source_model, alias)? {
        return Ok(());
    }
    if let Some(definition) = oauth_model_definitions.iter().find(|definition| {
        definition
            .models
            .iter()
            .any(|model| model.id.eq_ignore_ascii_case(source_model))
    }) {
        append_managed_oauth_model_alias(
            root,
            definition.channel.key,
            source_model,
            alias,
            definition.channel.force_mapping,
        )?;
        return Ok(());
    }
    Err(format!(
        "无法确定模型 {source_model} 的 CPA 配置来源，无法创建 Claude Desktop 别名 {alias}"
    ))
}

pub(crate) fn configured_model_client_identity(
    root: &serde_norway::Mapping,
    client_model: &str,
) -> Option<(String, String)> {
    for section in MODEL_ALIAS_CONFIG_SECTIONS {
        let Some(providers) =
            yaml_mapping_value(root, section).and_then(serde_norway::Value::as_sequence)
        else {
            continue;
        };
        for provider in providers {
            let Some(provider) = provider.as_mapping() else {
                continue;
            };
            let Some(models) =
                yaml_mapping_value(provider, "models").and_then(serde_norway::Value::as_sequence)
            else {
                continue;
            };
            for model in models {
                let Some((source, configured_client_model, _)) = configured_model_identity(model)
                else {
                    continue;
                };
                if configured_client_model.eq_ignore_ascii_case(client_model) {
                    return Some((source, configured_client_model));
                }
            }
        }
    }
    if let Some(oauth_aliases) =
        yaml_mapping_value(root, "oauth-model-alias").and_then(serde_norway::Value::as_mapping)
    {
        for entries in oauth_aliases
            .values()
            .filter_map(serde_norway::Value::as_sequence)
        {
            for model in entries {
                let Some((source, configured_client_model, _)) = configured_model_identity(model)
                else {
                    continue;
                };
                if configured_client_model.eq_ignore_ascii_case(client_model) {
                    return Some((source, configured_client_model));
                }
            }
        }
    }
    None
}

pub(crate) fn append_claude_desktop_model_alias(
    root: &mut serde_norway::Mapping,
    source_model: &str,
    alias: &str,
) -> Result<bool, String> {
    for section in MODEL_ALIAS_CONFIG_SECTIONS {
        let Some(providers) = yaml_mapping_value_mut(root, section) else {
            continue;
        };
        let providers = providers
            .as_sequence_mut()
            .ok_or_else(|| format!("{section} 必须是数组"))?;
        for provider in providers {
            let Some(provider) = provider.as_mapping_mut() else {
                continue;
            };
            let Some(models) = yaml_mapping_value_mut(provider, "models") else {
                continue;
            };
            let models = models
                .as_sequence_mut()
                .ok_or_else(|| format!("{section}.models 必须是数组"))?;
            let Some(source) = models.iter().find_map(|model| {
                let (upstream_model, client_model, _) = configured_model_identity(model)?;
                client_model
                    .eq_ignore_ascii_case(source_model)
                    .then(|| (model.clone(), upstream_model))
            }) else {
                continue;
            };
            let (source, upstream_model) = source;
            let mut alias_model = source.as_mapping().cloned().unwrap_or_else(|| {
                let mut mapping = serde_norway::Mapping::new();
                mapping.insert(
                    yaml_key("name"),
                    serde_norway::Value::String(upstream_model.clone()),
                );
                mapping
            });
            alias_model.insert(
                yaml_key("name"),
                serde_norway::Value::String(upstream_model),
            );
            alias_model.insert(
                yaml_key("alias"),
                serde_norway::Value::String(alias.to_string()),
            );
            let display_name = managed_claude_alias_display_name(alias)
                .ok_or_else(|| format!("不支持的 Claude 托管别名: {alias}"))?;
            alias_model.insert(
                yaml_key("display-name"),
                serde_norway::Value::String(display_name.to_string()),
            );
            models.push(serde_norway::Value::Mapping(alias_model));
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn append_managed_oauth_model_alias(
    root: &mut serde_norway::Mapping,
    channel: &str,
    source_model: &str,
    alias: &str,
    force_mapping: bool,
) -> Result<(), String> {
    let oauth_aliases = root
        .entry(yaml_key("oauth-model-alias"))
        .or_insert_with(|| serde_norway::Value::Mapping(serde_norway::Mapping::new()))
        .as_mapping_mut()
        .ok_or_else(|| "oauth-model-alias 必须是 YAML 映射".to_string())?;
    let channel_aliases = oauth_aliases
        .entry(yaml_key(channel))
        .or_insert_with(|| serde_norway::Value::Sequence(Vec::new()))
        .as_sequence_mut()
        .ok_or_else(|| format!("oauth-model-alias.{channel} 必须是数组"))?;
    let display_name = managed_claude_alias_display_name(alias)
        .ok_or_else(|| format!("不支持的 Claude 托管别名: {alias}"))?;
    let mut alias_mapping = serde_norway::Mapping::new();
    alias_mapping.insert(
        yaml_key("name"),
        serde_norway::Value::String(source_model.to_string()),
    );
    alias_mapping.insert(
        yaml_key("alias"),
        serde_norway::Value::String(alias.to_string()),
    );
    alias_mapping.insert(yaml_key("fork"), serde_norway::Value::Bool(true));
    if force_mapping {
        alias_mapping.insert(yaml_key("force-mapping"), serde_norway::Value::Bool(true));
    }
    alias_mapping.insert(
        yaml_key("display-name"),
        serde_norway::Value::String(display_name.to_string()),
    );
    channel_aliases.push(serde_norway::Value::Mapping(alias_mapping));
    Ok(())
}

pub(crate) async fn fetch_oauth_channel_model_definitions(
    config: &GuiConfigFile,
    channel: &str,
) -> Result<Vec<CodexModelDefinition>, String> {
    let client = management_http_client()?;
    let response = client
        .get(management_endpoint(
            config,
            &format!("model-definitions/{channel}"),
        )?)
        .header("Authorization", management_authorization(config)?)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            format_management_request_error(&format!("读取 {channel} OAuth 模型定义失败"), &error)
        })?;
    let payload = read_management_value(response).await?;
    parse_codex_model_definitions(&payload)
}

pub(crate) async fn fetch_oauth_model_definitions(
    config: &GuiConfigFile,
) -> Vec<OAuthModelDefinitions> {
    let active_channels = fetch_active_oauth_alias_channels(config).await.ok();
    let mut definitions = Vec::new();
    for channel in OAUTH_ALIAS_CHANNELS {
        if active_channels
            .as_ref()
            .is_some_and(|active| !active.contains(channel.key))
        {
            continue;
        }
        if let Ok(models) = fetch_oauth_channel_model_definitions(config, channel.key).await {
            definitions.push(OAuthModelDefinitions { channel, models });
        }
    }
    definitions
}

pub(crate) fn configured_model_identity(
    model: &serde_norway::Value,
) -> Option<(String, String, Option<String>)> {
    if let Some(name) = model
        .as_str()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        return Some((name.to_string(), name.to_string(), None));
    }
    let model = model.as_mapping()?;
    let name = yaml_mapping_value(model, "name")
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())?;
    let alias = yaml_mapping_value(model, "alias")
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|alias| !alias.is_empty());
    let display_name = yaml_mapping_value(model, "display-name")
        .or_else(|| yaml_mapping_value(model, "display_name"))
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some((
        name.to_string(),
        alias.unwrap_or(name).to_string(),
        display_name,
    ))
}

pub(crate) fn render_updated_core_yaml(
    document: &mut yaml_serde_edit::YamlValue,
    updated: serde_norway::Value,
) -> Result<String, String> {
    document.set(updated);
    let rendered = expand_top_level_flow_style_collections(&document.get_string(), document.get())?;
    let rendered = indent_indentationless_yaml_sequences(&rendered);
    serde_norway::from_str::<serde_norway::Value>(&rendered)
        .map_err(|error| format!("验证更新后的内核配置失败: {error}"))?;
    Ok(rendered)
}

pub(crate) fn expand_top_level_flow_style_collections(
    content: &str,
    document: &serde_norway::Value,
) -> Result<String, String> {
    let root = document
        .as_mapping()
        .ok_or_else(|| "内核配置顶层必须是 YAML 映射".to_string())?;
    let mut rendered = content.to_string();
    for (key, value) in root {
        let Some(key) = key.as_str() else {
            continue;
        };
        let is_non_empty_collection = match value {
            serde_norway::Value::Mapping(mapping) => !mapping.is_empty(),
            serde_norway::Value::Sequence(sequence) => !sequence.is_empty(),
            _ => false,
        };
        if !is_non_empty_collection || !top_level_yaml_entry_uses_flow_style(&rendered, key) {
            continue;
        }
        let mut wrapper = serde_norway::Mapping::new();
        wrapper.insert(yaml_key(key), value.clone());
        let block = serde_norway::to_string(&serde_norway::Value::Mapping(wrapper))
            .map_err(|error| format!("格式化内核 YAML 配置失败: {error}"))?;
        rendered = replace_top_level_yaml_block(&rendered, key, &block);
    }
    Ok(rendered)
}

pub(crate) fn top_level_yaml_entry_uses_flow_style(content: &str, key: &str) -> bool {
    let prefix = format!("{key}:");
    yaml_line_ranges(content).into_iter().any(|range| {
        let line = yaml_line_content(content, range);
        if line.chars().next().is_some_and(char::is_whitespace) {
            return false;
        }
        line.strip_prefix(&prefix)
            .map(str::trim_start)
            .is_some_and(|value| value.starts_with('[') || value.starts_with('{'))
    })
}

pub(crate) fn indent_indentationless_yaml_sequences(content: &str) -> String {
    let mut lines = content
        .split_inclusive('\n')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !content.is_empty() && !content.ends_with('\n') && lines.is_empty() {
        lines.push(content.to_string());
    }

    loop {
        let mut range_to_indent = None;
        for key_index in 0..lines.len() {
            let key_line = lines[key_index].trim_end_matches(['\r', '\n']);
            let key_trimmed = key_line.trim_start_matches(' ');
            if key_trimmed.is_empty()
                || key_trimmed.starts_with('#')
                || key_trimmed.starts_with('-')
                || !key_trimmed.ends_with(':')
            {
                continue;
            }
            let parent_indent = key_line.len() - key_trimmed.len();
            let Some(first_item) = ((key_index + 1)..lines.len()).find(|index| {
                let line = lines[*index].trim_end_matches(['\r', '\n']).trim();
                !line.is_empty() && !line.starts_with('#')
            }) else {
                continue;
            };
            let first_line = lines[first_item].trim_end_matches(['\r', '\n']);
            let first_trimmed = first_line.trim_start_matches(' ');
            let first_indent = first_line.len() - first_trimmed.len();
            if first_indent != parent_indent
                || !is_indentationless_yaml_sequence_item(first_trimmed)
            {
                continue;
            }

            let mut end = first_item;
            while end < lines.len() {
                let line = lines[end].trim_end_matches(['\r', '\n']);
                let trimmed = line.trim_start_matches(' ');
                if !trimmed.is_empty() {
                    let indent = line.len() - trimmed.len();
                    if indent < parent_indent
                        || (indent == parent_indent
                            && !is_indentationless_yaml_sequence_item(trimmed))
                    {
                        break;
                    }
                }
                end += 1;
            }
            range_to_indent = Some((first_item, end));
            break;
        }

        let Some((start, end)) = range_to_indent else {
            break;
        };
        for line in &mut lines[start..end] {
            if !line.trim().is_empty() {
                line.insert_str(0, "  ");
            }
        }
    }
    lines.concat()
}

pub(crate) fn truncate_for_error(value: &str) -> String {
    const LIMIT: usize = 240;
    let trimmed = value.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_string();
    }
    let shortened: String = trimmed.chars().take(LIMIT).collect();
    format!("{shortened}…")
}

pub(crate) fn open_external_url_inner(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("链接为空".to_string());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("只允许打开 http/https 链接".to_string());
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|err| format!("打开浏览器失败: {err}"))
}
