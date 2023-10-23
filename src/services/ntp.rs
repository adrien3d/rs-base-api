use actix_web::{
    rt::{time::interval, Runtime},
    web,
};
use chrono::{DateTime, Utc};
use futures::{pin_mut, prelude::*, select};
use std::{sync::Mutex, time::Duration};
use stoppable_thread::{SimpleAtomicBool, StoppableHandle};

use crate::ProgramAppState;

const NTP_REFRESH_INTERVAL: Duration = Duration::from_secs(3600);
const STOPPING_INTERVAL: Duration = Duration::from_secs(1);
pub struct Ntp {
    local_time_offset: Mutex<chrono::Duration>,
}

impl Default for Ntp {
    fn default() -> Self {
        Self::new()
    }
}

impl Ntp {
    /// This will create a new updated instance of NTP, which will block.
    pub fn new() -> Self {
        let instance = Ntp {
            local_time_offset: Mutex::new(chrono::Duration::zero()),
        };
        instance.update_offset();
        instance
    }

    pub fn start_time_thread(&self, state: web::Data<ProgramAppState>) -> StoppableHandle<()> {
        stoppable_thread::spawn(|stopped| {
            Runtime::new()
                .unwrap()
                .block_on(Ntp::ntp_update_loop(stopped, state))
        })
    }

    async fn ntp_update_loop(stopped: &SimpleAtomicBool, state: web::Data<ProgramAppState>) {
        let mut ntp_update_interval = interval(NTP_REFRESH_INTERVAL);
        let mut stopped_interval = interval(STOPPING_INTERVAL);
        while !stopped.get() {
            let ntp_update_tick = ntp_update_interval.tick().fuse();
            let stopped_tick = stopped_interval.tick().fuse();
            pin_mut!(ntp_update_tick, stopped_tick);
            select! {
                _ = ntp_update_tick => {
                    state.ntp.update_offset();
                },
                _ = stopped_tick => {}
            }
        }
    }

    /// Updates Ntp offset from one of the servers.
    fn update_offset(&self) {
        const NTP_SERVERS: &[&str] = &[
            "pool.ntp.org:123",
            "time.nist.gov:123",
            "time.google.com:123",
            "time.windows.com:123",
            "ntp.ubuntu.com:123",
        ];

        log::debug!("Updating NTP time");
        let client = rsntp::SntpClient::new();

        let mut sync_res = None;
        for server in NTP_SERVERS {
            match client.synchronize(server) {
                Ok(res) => {
                    sync_res = Some(res);
                    break;
                }
                Err(error) => log::warn!("Ntp update from {server} failed: {error:?}"),
            }
        }
        let Some(sync_res) = sync_res else {
            log::error!("NTP update failed from all servers");
            return;
        };

        let sntp_res = sync_res.clock_offset();
        let offset_duration = sntp_res.into_chrono_duration().unwrap() * sntp_res.signum();

        log::info!("Offset from NTP is: {offset_duration}");

        let Ok(mut offset) = self.local_time_offset.lock() else {
            log::warn!("Failure of locking offset while setting current_time");
            return;
        };
        *offset = offset_duration;
    }

    pub fn current_time(&self) -> DateTime<Utc> {
        let Ok(offset) = self.local_time_offset.lock() else {
            log::error!("Failure of locking offset while getting current_time");
            return Utc::now();
        };
        Utc::now().checked_add_signed(*offset).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::ntp::Ntp;
    use chrono::{Duration, Utc};
    use test_log::test;

    #[test]
    fn test_ntp_offset_update() {
        let ntp = Ntp::new();
        let mut guard = ntp.local_time_offset.lock().unwrap();

        *guard = Duration::max_value();
        drop(guard);

        ntp.update_offset();
        let guard = ntp.local_time_offset.lock().unwrap();
        let updated_value = *guard;

        assert!(
            updated_value != Duration::max_value(),
            "`ntp.update_offset` should update the ntp offset"
        );
    }

    #[test]
    fn test_ntp_current_time() {
        let ntp = Ntp::new();

        for offset in [Duration::hours(-12), Duration::zero(), Duration::hours(12)].iter() {
            let mut guard = ntp.local_time_offset.lock().unwrap();
            *guard = *offset;
            drop(guard);

            let ntp_dt = ntp.current_time();
            let now_dt = Utc::now();

            let dt_diff = ntp_dt - now_dt;
            assert!(
                dt_diff > *offset - Duration::seconds(1),
                "`ntp.current_time` should use the ntp offset"
            );
            assert!(
                dt_diff < *offset + Duration::seconds(1),
                "`ntp.current_time` should use the ntp offset"
            );
        }
    }

    #[test]
    fn test_ntp_current_time_with_poisoned_mutex() {
        let ntp = Ntp::new();
        let _ = std::panic::catch_unwind(|| {
            let mut guard = ntp.local_time_offset.lock().unwrap();
            *guard = Duration::hours(12);
            panic!(); // Poison the mutex.
        });
        let ntp_dt = ntp.current_time();
        let now_dt = Utc::now();

        let dt_diff = ntp_dt - now_dt;
        assert!(
            dt_diff > Duration::seconds(-1),
            "`ntp.current_time` should fall back to system UTC time if the offset mutex is stuck"
        );
        assert!(
            dt_diff < Duration::seconds(1),
            "`ntp.current_time` should fall back to system UTC time if the offset mutex is stuck"
        );
    }
}
