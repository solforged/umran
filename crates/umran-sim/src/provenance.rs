//! Facts recorded by the mechanism that moved a word, never inferred from its form.
use crate::{Contact, ContactKind, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoanCause {
    Contact {
        donor: usize,
        recipient: usize,
        kind: ContactKind,
        since: u32,
    },
    Rule {
        ruler: usize,
        ruled: usize,
        state: Option<usize>,
    },
    Faith {
        religion: usize,
        teacher: Option<usize>,
        recipient: usize,
    },
    Shift {
        community: usize,
        from_variety: usize,
    },
    City {
        city: usize,
        community: usize,
    },
    /// Learned borrowing from the frozen language selected by the channel.
    Classical {
        classical: usize,
        recipient: usize,
    },
    /// Finding a word for a newly held idea, from the selected partner.
    Coinage {
        donor: usize,
        recipient: usize,
    },
    /// The writer has no recorded mechanism.
    Unrecorded,
}

impl World {
    pub(crate) fn contact_loan_cause(
        &self,
        contact: &Contact,
        donor: usize,
        recipient: usize,
    ) -> LoanCause {
        if contact.kind == ContactKind::Rule {
            LoanCause::Rule {
                ruler: contact.a,
                ruled: contact.b,
                state: self.rules(contact.a),
            }
        } else {
            // Religion contacts need not belong to any founded religion.
            LoanCause::Contact {
                donor,
                recipient,
                kind: contact.kind,
                since: contact.since,
            }
        }
    }
}
