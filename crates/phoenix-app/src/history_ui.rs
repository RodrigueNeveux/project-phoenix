//! UI side of the message cache: apply a background refresh without moving the view.

use gpui::{Context, ListOffset};

use crate::{
    app::PhoenixApp,
    model::{Id, Message},
};

/// `AppEvent::HistoryRefresh`: the cached first page shown for `channel` was
/// outdated. Replace it, keeping the message at the top of the viewport in
/// place when the user has scrolled up.
pub fn on_refresh(app: &mut PhoenixApp, channel: Id, messages: Vec<Message>, cx: &mut Context<PhoenixApp>) {
    // Only a still-first page shown from the cache is replaced.
    if !app.messages.get(&channel).is_some_and(|e| e.loaded_once && !e.loading && e.msgs.len() <= 50) {
        return;
    }
    let anchor = (app.selected_channel == Some(channel) && !app.chat.at_bottom).then(|| {
        let top = app.message_list.logical_scroll_top();
        // Row 0 is the channel header; message i is row i + 1.
        let id = top.item_ix.checked_sub(1).and_then(|i| app.messages.get(&channel)?.msgs.get(i).map(|m| m.id));
        (id, top.offset_in_item)
    });
    app.messages.remove(&channel);
    app.insert_history(channel, messages, false, cx);
    if let Some((Some(id), offset_in_item)) = anchor
        && let Some(ix) = app.messages.get(&channel).and_then(|e| e.index_of(id))
    {
        app.message_list.scroll_to(ListOffset { item_ix: ix + 1, offset_in_item });
    }
}
