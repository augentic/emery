//! The master anchor
//!
//! Decides which revision a `specify` run continues. A project that carries
//! its master beside the code hands it in; the run reads it, refuses another
//! grammar's or one that is not a master, and makes it the current revision so
//! the requirements inherit its ids and the diff is drawn against it. A run
//! carrying nothing continues the store's current revision — and when that is
//! outdated or unreadable, nothing: regeneration is the way out, never a
//! dead end.

use omnia_guest::{BlobStore, Error, StateStore};

use crate::artifact::Dossier;
use crate::specify::Carried;
use crate::store;

// Anchors the run: the carried master, adopted as current, or the store's
// readable current revision, or nothing.
pub async fn anchor<P: StateStore + BlobStore>(
    provider: &P, carried: Option<Carried>,
) -> Result<Option<Dossier>, Error> {
    let Some(Carried { spec, design }) = carried else {
        return Ok(store::current(provider).await.ok().flatten());
    };

    let dossier = Dossier::read(spec, design)?;
    store::adopt(provider, &dossier).await?;
    tracing::debug!(revision = %dossier.revision(), "carried master adopted");

    Ok(Some(dossier))
}
