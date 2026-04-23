use cursive::{
    view::Nameable,
    views::{LinearLayout, TextView},
    Cursive,
};
use intersect_core::{
    documents::{AccountDocument, FragmentDocument, FragmentView, IndexDocument},
    Intersect, OpenDocument,
};

use crate::prompt::{unlock_trace, Prompt};

use super::{subview, Panel};

pub struct IndexPanel {
    pub doc: OpenDocument<IndexDocument>,
    pub fragment: Option<FragmentView>,
    pub author: Option<OpenDocument<AccountDocument>>,
}

impl IndexPanel {
    /// constructs the panel, fetching the fragment and opening the author doc.
    /// soft-fails on downstream errors, returning them separately so the caller
    /// can surface them without preventing the panel from opening.
    pub async fn new(
        doc: OpenDocument<IndexDocument>,
        intersect: &Intersect,
        prompt: &impl Prompt,
    ) -> (Self, Vec<String>) {
        let mut errors = Vec::new();

        // borrow in its own block so doc isn't held when we might move it below
        let view_result = {
            let borrowed = doc.updates.borrow();
            borrowed
                .as_ref()
                .map(|v| v.clone())
                .map_err(|e| format!("{e}"))
        };
        let view = match view_result {
            Ok(view) => view,
            Err(e) => {
                // no view at all, nothing to fetch downstream
                errors.push(format!("index read failed: {e}"));
                return (
                    Self {
                        doc,
                        fragment: None,
                        author: None,
                    },
                    errors,
                );
            }
        };

        // fragment is immutable, so just fetch it
        let fragment = if let Some(trace) = view.fragment() {
            let result: anyhow::Result<_> = async {
                let opened = trace.clone().into_typed::<FragmentDocument>()?;
                let r = unlock_trace(opened, prompt).await?;
                Ok(intersect.fetch(&r).await?)
            }
            .await;
            match result {
                Ok(fv) => Some(fv),
                Err(e) => {
                    errors.push(format!("fragment: {e:#}"));
                    None
                }
            }
        } else {
            None
        };

        // author is mutable, so open it for ongoing updates
        let author = if let Some(trace) = view.author() {
            let result: anyhow::Result<_> = async {
                let opened = trace.clone().into_typed::<AccountDocument>()?;
                let r = unlock_trace(opened, prompt).await?;
                Ok(intersect.open(&r).await?)
            }
            .await;
            match result {
                Ok(doc) => Some(doc),
                Err(e) => {
                    errors.push(format!("author: {e:#}"));
                    None
                }
            }
        } else {
            None
        };

        (
            Self {
                doc,
                fragment,
                author,
            },
            errors,
        )
    }
}

impl Panel for IndexPanel {
    fn title(&self) -> String {
        match &*self.doc.updates.borrow() {
            Ok(view) => format!("index: {}", view.name().as_ref()),
            Err(_) => "index: ?".to_string(),
        }
    }

    fn has_updates(&self) -> bool {
        self.doc.updates.has_changed().unwrap_or(false)
            || self
                .author
                .as_ref()
                .map(|a| a.updates.has_changed().unwrap_or(false))
                .unwrap_or(false)
    }

    fn build_view(&mut self, id: usize) -> Box<dyn cursive::View> {
        let mut layout = LinearLayout::vertical();

        layout
            .add_child(TextView::new(render_index(&mut self.doc)).with_name(subview(id, "index")));

        if let Some(author) = self.author.as_mut() {
            layout.add_child(TextView::new("\n── author ──"));
            layout.add_child(TextView::new(render_author(author)).with_name(subview(id, "author")));
        }

        if let Some(fragment) = &self.fragment {
            layout.add_child(TextView::new("\n── content ──"));
            // fragment is immutable — no name needed, never updated
            layout.add_child(TextView::new(format!("{fragment}")));
        }

        Box::new(layout)
    }

    fn make_update(&mut self, id: usize) -> Box<dyn FnOnce(&mut Cursive)> {
        let index_content = render_index(&mut self.doc);
        let author_content = self.author.as_mut().map(render_author);

        let index_name = subview(id, "index");
        let author_name = subview(id, "author");

        Box::new(move |s| {
            s.call_on_name(&index_name, |v: &mut TextView| v.set_content(index_content));
            if let Some(content) = author_content {
                s.call_on_name(&author_name, |v: &mut TextView| v.set_content(content));
            }
        })
    }
}

fn render_index(doc: &mut OpenDocument<IndexDocument>) -> String {
    match &*doc.updates.borrow_and_update() {
        Ok(view) => format!("{view}"),
        Err(e) => format!("error: {e}"),
    }
}

fn render_author(doc: &mut OpenDocument<AccountDocument>) -> String {
    match &*doc.updates.borrow_and_update() {
        Ok(view) => format!("{view}"),
        Err(e) => format!("(author unavailable: {e})"),
    }
}
