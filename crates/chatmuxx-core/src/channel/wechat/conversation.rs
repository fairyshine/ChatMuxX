use crate::channel::wechat::model::WeChatMessage;

pub fn extract_text(message: &WeChatMessage) -> Option<String> {
    for item in &message.item_list {
        if item.r#type == Some(1) {
            if let Some(text) = item.text_item.as_ref().and_then(|item| item.text.as_ref()) {
                return Some(text.clone());
            }
        }

        if item.r#type == Some(3) {
            if let Some(text) = item.voice_item.as_ref().and_then(|item| item.text.as_ref()) {
                return Some(text.clone());
            }
        }
    }

    None
}

pub fn conversation_external_id(message: &WeChatMessage) -> Option<String> {
    non_empty(message.group_id.as_deref())
        .or_else(|| non_empty(message.from_user_id.as_deref()))
        .map(str::to_owned)
}

pub fn conversation_id(account_id: &str, message: &WeChatMessage) -> Option<String> {
    conversation_external_id(message).map(|external| {
        if is_group_message(message) {
            format!("conv_wechat_{account_id}_group_{external}")
        } else {
            format!("conv_wechat_{account_id}_direct_{external}")
        }
    })
}

pub fn is_group_message(message: &WeChatMessage) -> bool {
    non_empty(message.group_id.as_deref()).is_some()
}

pub fn token_ref(account_id: &str, conversation_id: &str) -> String {
    format!("ctx_{account_id}_{conversation_id}")
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::wechat::model::{WeChatMessageItem, WeChatTextItem};

    #[test]
    fn text_item_is_extracted() {
        let message = WeChatMessage {
            message_id: None,
            from_user_id: Some("alice".to_owned()),
            to_user_id: None,
            group_id: None,
            create_time_ms: None,
            message_type: None,
            message_state: None,
            item_list: vec![WeChatMessageItem {
                r#type: Some(1),
                text_item: Some(WeChatTextItem {
                    text: Some("hello".to_owned()),
                }),
                voice_item: None,
            }],
            context_token: None,
        };

        assert_eq!(extract_text(&message), Some("hello".to_owned()));
        assert_eq!(
            conversation_id("acct", &message),
            Some("conv_wechat_acct_direct_alice".to_owned())
        );
    }

    #[test]
    fn empty_group_id_falls_back_to_direct_sender() {
        let message = WeChatMessage {
            message_id: None,
            from_user_id: Some("alice".to_owned()),
            to_user_id: None,
            group_id: Some(String::new()),
            create_time_ms: None,
            message_type: None,
            message_state: None,
            item_list: Vec::new(),
            context_token: None,
        };

        assert!(!is_group_message(&message));
        assert_eq!(
            conversation_id("acct", &message),
            Some("conv_wechat_acct_direct_alice".to_owned())
        );
    }
}
