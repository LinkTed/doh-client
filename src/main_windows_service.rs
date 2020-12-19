use doh_client::{get_app, run, Config};
use lazy_static::lazy_static;
use log::{error, info, set_max_level, LevelFilter};
use std::ffi::OsString;
use std::sync::Mutex;
use std::time::Duration;
use tokio::runtime::Runtime;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{
    register, ServiceControlHandlerResult, ServiceStatusHandle,
};
use windows_service::{define_windows_service, service_dispatcher};

define_windows_service!(ffi_windows_service_doh_client, windows_service_doh_client);

lazy_static! {
    static ref STATUS_HANDLE: Mutex<Option<ServiceStatusHandle>> = Mutex::new(None);
    static ref RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
    static ref TIMEOUT: Duration = Duration::from_secs(20_000);
}

fn windows_service_event_handler(control_event: ServiceControl) -> ServiceControlHandlerResult {
    match control_event {
        ServiceControl::Stop => {
            info!("service control stop");
            let status_handle_lock = STATUS_HANDLE.lock().unwrap();
            if let Some(status_handle) = status_handle_lock.as_ref() {
                if RUNTIME.lock().unwrap().take().is_some() {
                    let service_status_stop_pending = ServiceStatus {
                        service_type: ServiceType::OWN_PROCESS,
                        current_state: ServiceState::StopPending,
                        controls_accepted: ServiceControlAccept::STOP,
                        exit_code: ServiceExitCode::Win32(0),
                        checkpoint: 0,
                        wait_hint: *TIMEOUT,
                        process_id: None,
                    };

                    status_handle
                        .set_service_status(service_status_stop_pending)
                        .unwrap();
                }

                let service_status_stop = ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::Stopped,
                    controls_accepted: ServiceControlAccept::STOP,
                    exit_code: ServiceExitCode::Win32(0),
                    checkpoint: 0,
                    wait_hint: Duration::default(),
                    process_id: None,
                };
                status_handle
                    .set_service_status(service_status_stop)
                    .unwrap();
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }
}

fn windows_service_doh_client(arguments: Vec<OsString>) {
    info!("windows service is started");
    let app = get_app();
    let matches = app.get_matches_from(arguments);

    let mut runtime = Runtime::new().unwrap();
    let config = runtime.block_on(Config::try_from(matches)).unwrap();

    let status_handle = register("doh-client", windows_service_event_handler).unwrap();
    info!("windows service event handler is registered");

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(next_status).unwrap();

    runtime.spawn(async {
        if let Err(e) = run(config).await {
            error!("doh-client stopped: {}", e);
            let status_handle_lock = STATUS_HANDLE.lock().unwrap();
            let service_status_stop = ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::STOP,
                exit_code: ServiceExitCode::ServiceSpecific(1),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            };
            status_handle_lock
                .unwrap()
                .set_service_status(service_status_stop)
                .unwrap();
        }
    });

    STATUS_HANDLE.lock().unwrap().replace(status_handle);
    RUNTIME.lock().unwrap().replace(runtime);
}

fn main() -> Result<(), windows_service::Error> {
    winlog::init("doh-client").unwrap();
    set_max_level(LevelFilter::Info);
    service_dispatcher::start("doh-client", ffi_windows_service_doh_client)
}
