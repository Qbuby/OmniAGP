use omni_core::UserQuota;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

pub struct BillingManager {
    quotas: DashMap<Uuid, UserQuota>,
    total_gpu_minutes: AtomicU64,
}

impl BillingManager {
    pub fn new() -> Self {
        Self {
            quotas: DashMap::new(),
            total_gpu_minutes: AtomicU64::new(0),
        }
    }

    pub fn set_quota(&self, user_id: Uuid, free_minutes: f64, paid_minutes: f64) {
        self.quotas.insert(user_id, UserQuota {
            user_id,
            free_minutes_remaining: free_minutes,
            paid_minutes_remaining: paid_minutes,
            total_used_minutes: 0.0,
        });
    }

    pub fn check_quota(&self, user_id: Uuid) -> bool {
        match self.quotas.get(&user_id) {
            Some(q) => q.free_minutes_remaining > 0.0 || q.paid_minutes_remaining > 0.0,
            None => true,
        }
    }

    pub fn deduct(&self, user_id: Uuid, gpu_minutes: f64) {
        if let Some(mut quota) = self.quotas.get_mut(&user_id) {
            quota.total_used_minutes += gpu_minutes;
            if quota.free_minutes_remaining >= gpu_minutes {
                quota.free_minutes_remaining -= gpu_minutes;
            } else {
                let remaining = gpu_minutes - quota.free_minutes_remaining;
                quota.free_minutes_remaining = 0.0;
                quota.paid_minutes_remaining = (quota.paid_minutes_remaining - remaining).max(0.0);
            }
        }
    }

    pub fn get_quota(&self, user_id: Uuid) -> Option<UserQuota> {
        self.quotas.get(&user_id).map(|q| q.clone())
    }

    pub async fn record_usage(&self, gpu_minutes: f64) {
        let bits = (gpu_minutes * 1000.0) as u64;
        self.total_gpu_minutes.fetch_add(bits, Ordering::Relaxed);
    }

    pub fn total_gpu_minutes(&self) -> f64 {
        self.total_gpu_minutes.load(Ordering::Relaxed) as f64 / 1000.0
    }
}
