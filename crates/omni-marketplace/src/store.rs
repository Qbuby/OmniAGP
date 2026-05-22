use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;
use uuid::Uuid;

use crate::asset_pack::{AssetPack, AssetPackManifest};
use crate::community::{Contribution, ContributionStatus, ContributionType, ContributorProfile};
use crate::search::{SearchFilter, SearchResult, SortBy};

pub struct MarketplaceStore {
    storage_dir: PathBuf,
    asset_packs: HashMap<Uuid, AssetPack>,
    contributors: HashMap<Uuid, ContributorProfile>,
    contributions: Vec<Contribution>,
}

impl MarketplaceStore {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            asset_packs: HashMap::new(),
            contributors: HashMap::new(),
            contributions: Vec::new(),
        }
    }

    pub fn upload_asset_pack(
        &mut self,
        manifest: AssetPackManifest,
        file_path: &Path,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let dest_dir = self.storage_dir.join("packs").join(id.to_string());
        std::fs::create_dir_all(&dest_dir)?;

        let dest_file = dest_dir.join(format!("{}.omnipak", manifest.name));
        std::fs::copy(file_path, &dest_file).context("failed to copy asset pack")?;

        let pack = AssetPack {
            id,
            manifest,
            download_url: dest_file.to_string_lossy().to_string(),
            download_count: 0,
            rating: 0.0,
            rating_count: 0,
        };

        self.asset_packs.insert(id, pack);
        info!(pack_id = %id, "asset pack uploaded");
        Ok(id)
    }

    pub fn download_asset_pack(&mut self, pack_id: Uuid, dest_dir: &Path) -> Result<PathBuf> {
        let pack = self
            .asset_packs
            .get_mut(&pack_id)
            .context("asset pack not found")?;

        let source = Path::new(&pack.download_url);
        let dest = dest_dir.join(source.file_name().unwrap_or_default());
        std::fs::copy(source, &dest).context("failed to download asset pack")?;

        pack.download_count += 1;
        Ok(dest)
    }

    pub fn search_asset_packs(&self, filter: &SearchFilter) -> SearchResult<AssetPack> {
        let mut results: Vec<&AssetPack> = self.asset_packs.values().collect();

        if let Some(ref query) = filter.query {
            let q = query.to_lowercase();
            results.retain(|p| {
                p.manifest.name.to_lowercase().contains(&q)
                    || p.manifest.description.to_lowercase().contains(&q)
                    || p.manifest.tags.iter().any(|t| t.to_lowercase().contains(&q))
            });
        }

        if let Some(ref asset_type) = filter.asset_type {
            results.retain(|p| &p.manifest.asset_type == asset_type);
        }

        if !filter.tags.is_empty() {
            results.retain(|p| filter.tags.iter().any(|t| p.manifest.tags.contains(t)));
        }

        if let Some(min_rating) = filter.min_rating {
            results.retain(|p| p.rating >= min_rating);
        }

        match filter.sort_by {
            SortBy::Downloads => results.sort_by(|a, b| b.download_count.cmp(&a.download_count)),
            SortBy::Rating => results.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Newest => results.sort_by(|a, b| b.id.cmp(&a.id)),
            _ => {}
        }

        let total = results.len() as u64;
        let start = (filter.page * filter.per_page) as usize;
        let end = std::cmp::min(start + filter.per_page as usize, results.len());
        let page_items = if start < results.len() {
            results[start..end].iter().map(|p| (*p).clone()).collect()
        } else {
            vec![]
        };

        SearchResult {
            items: page_items,
            total,
            page: filter.page,
            per_page: filter.per_page,
            has_more: end < results.len(),
        }
    }

    pub fn rate_asset_pack(&mut self, pack_id: Uuid, score: f32) -> Result<()> {
        let pack = self
            .asset_packs
            .get_mut(&pack_id)
            .context("asset pack not found")?;

        let total_score = pack.rating * pack.rating_count as f32 + score;
        pack.rating_count += 1;
        pack.rating = total_score / pack.rating_count as f32;
        Ok(())
    }

    pub fn submit_contribution(
        &mut self,
        contributor_id: Uuid,
        contribution_type: ContributionType,
        name: String,
        version: String,
    ) -> Contribution {
        let contribution = Contribution {
            id: Uuid::new_v4(),
            contributor_id,
            contribution_type,
            name,
            version,
            status: ContributionStatus::Submitted,
            submitted_at: chrono::Utc::now(),
            reviewed_at: None,
            reviewer_notes: None,
        };
        self.contributions.push(contribution.clone());
        contribution
    }

    pub fn get_contributor(&self, id: Uuid) -> Option<&ContributorProfile> {
        self.contributors.get(&id)
    }

    pub fn register_contributor(&mut self, profile: ContributorProfile) {
        self.contributors.insert(profile.id, profile);
    }
}
