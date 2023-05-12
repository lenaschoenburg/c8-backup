use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::types::{BackupDescriptor, TakeBackupRequest};

use super::{Backup, BackupError, BackupId, Endpoint};
