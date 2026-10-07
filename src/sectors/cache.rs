// src/sectors/cache.rs — In-Memory Cache dengan TTL 24 jam

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, info};

/// Satu entri cache dengan data dan timestamp.
#[derive(Debug)]
struct CacheEntry<T> {
    data: T,
    inserted_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            inserted_at: Utc::now(),
        }
    }

    /// Apakah entri ini sudah kadaluarsa?
    fn is_expired(&self, ttl: Duration) -> bool {
        Utc::now() > self.inserted_at + ttl
    }
}

/// Cache in-memory generik dengan TTL.
///
/// Thread-safe via `RwLock` sehingga bisa dibungkus `Arc` dan dibagi antar task.
///
/// # Contoh
/// ```rust
/// let cache: MemCache<Vec<QuarterlyReport>> = MemCache::new(Duration::hours(24));
/// cache.set("BBCA", data);
/// if let Some(cached) = cache.get("BBCA") { /* gunakan */ }
/// ```
pub struct MemCache<T> {
    store: RwLock<HashMap<String, CacheEntry<T>>>,
    ttl: Duration,
}

impl<T: Clone + std::fmt::Debug> MemCache<T> {
    /// Membuat cache baru dengan TTL tertentu.
    pub fn new(ttl: Duration) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Membuat cache dengan TTL standar 24 jam.
    pub fn with_24h_ttl() -> Self {
        Self::new(Duration::hours(24))
    }

    /// Menyimpan data ke cache dengan key tertentu.
    pub fn set(&self, key: impl Into<String>, data: T) {
        let key = key.into();
        let mut store = self.store.write().expect("Cache write lock poisoned");
        debug!(cache_key = %key, "Cache SET");
        store.insert(key, CacheEntry::new(data));
    }

    /// Mengambil data dari cache.
    /// Mengembalikan `None` jika tidak ada atau sudah kadaluarsa.
    pub fn get(&self, key: &str) -> Option<T> {
        let store = self.store.read().expect("Cache read lock poisoned");
        let entry = store.get(key)?;

        if entry.is_expired(self.ttl) {
            debug!(cache_key = %key, "Cache MISS (expired)");
            return None;
        }

        debug!(cache_key = %key, "Cache HIT");
        Some(entry.data.clone())
    }

    /// Menghapus entri kadaluarsa dari cache (garbage collect manual).
    pub fn evict_expired(&self) {
        let mut store = self.store.write().expect("Cache write lock poisoned");
        let before = store.len();
        store.retain(|_, entry| !entry.is_expired(self.ttl));
        let evicted = before - store.len();
        if evicted > 0 {
            info!(evicted = evicted, "Cache eviction selesai");
        }
    }

    /// Jumlah entri aktif (termasuk yang belum di-evict).
    pub fn len(&self) -> usize {
        self.store.read().expect("Cache read lock poisoned").len()
    }

    /// Apakah cache kosong?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
