use cursive::{view::Nameable, views::TextView, Cursive};
use intersect_core::{documents::AccountDocument, OpenDocument};

use super::{subview, Panel};

pub struct AccountPanel {
    pub doc: OpenDocument<AccountDocument>,
}

impl Panel for AccountPanel {
    fn title(&self) -> String {
        match &*self.doc.updates.borrow() {
            Ok(view) => format!(
                "account: {}",
                view.name().map(|n| n.as_ref()).unwrap_or("?")
            ),
            Err(_) => "account: ?".to_string(),
        }
    }

    fn has_updates(&self) -> bool {
        self.doc.updates.has_changed().unwrap_or(false)
    }

    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View> {
        Box::new(TextView::new(render(&mut self.doc)).with_name(subview(id, "content")))
    }

    fn make_update(&mut self, id: usize) -> Box<dyn FnOnce(&mut Cursive)> {
        let content = render(&mut self.doc);
        let name = subview(id, "content");
        Box::new(move |s| {
            s.call_on_name(&name, |v: &mut TextView| v.set_content(content));
        })
    }
}

fn render(doc: &mut OpenDocument<AccountDocument>) -> String {
    match &*doc.updates.borrow_and_update() {
        Ok(view) => format!("{view}"),
        Err(e) => format!("error: {e}"),
    }
}
