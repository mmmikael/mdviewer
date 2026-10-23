//! HTTP through macOS' own URL loading (NSURLSession): native TLS, system
//! proxies and certificates, and no HTTP stack added to the app. Responses
//! arrive in pieces as they stream in.

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use super::providers::HttpRequest;

#[derive(Debug)]
pub enum Event {
    /// The HTTP status, sent once before any data.
    Status(u16),
    Data(Vec<u8>),
    /// The request finished; `Err` for network failures.
    Finished(Result<(), String>),
}

/// Starts a request. Dropping the receiver cancels it.
pub fn send(request: HttpRequest) -> Result<UnboundedReceiver<Event>, String> {
    let (tx, rx) = unbounded_channel();
    imp::send(request, tx)?;
    Ok(rx)
}

/// A whole (non-streamed) response.
pub async fn fetch(request: HttpRequest) -> Result<(u16, Vec<u8>), String> {
    let mut rx = send(request)?;
    let mut status = 0;
    let mut body = Vec::new();
    while let Some(event) = rx.recv().await {
        match event {
            Event::Status(code) => status = code,
            Event::Data(data) => body.extend(data),
            Event::Finished(result) => {
                result?;
                break;
            }
        }
    }
    Ok((status, body))
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use objc2::rc::Retained;
    use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
    use objc2::{AnyThread, DefinedClass, define_class, msg_send};
    use objc2_foundation::{
        NSData, NSError, NSHTTPURLResponse, NSMutableURLRequest, NSString, NSURL, NSURLSession,
        NSURLSessionConfiguration, NSURLSessionDataDelegate, NSURLSessionDataTask,
        NSURLSessionDelegate, NSURLSessionTask, NSURLSessionTaskDelegate,
    };

    use super::{Event, HttpRequest, UnboundedSender};

    /// Local models can think for a long time before the first token.
    const IDLE_TIMEOUT_SECS: f64 = 300.0;
    const NS_URL_ERROR_CANCELLED: isize = -999;

    pub struct Ivars {
        tx: UnboundedSender<Event>,
        sent_status: AtomicBool,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and this class
        // doesn't implement Drop.
        #[unsafe(super(NSObject))]
        #[name = "MDViewerStreamDelegate"]
        #[ivars = Ivars]
        struct StreamDelegate;

        unsafe impl NSObjectProtocol for StreamDelegate {}

        unsafe impl NSURLSessionDelegate for StreamDelegate {}

        unsafe impl NSURLSessionTaskDelegate for StreamDelegate {
            #[unsafe(method(URLSession:task:didCompleteWithError:))]
            fn did_complete(
                &self,
                _session: &NSURLSession,
                task: &NSURLSessionTask,
                error: Option<&NSError>,
            ) {
                self.send_status(task);
                let result = match error {
                    None => Ok(()),
                    Some(error) if error.code() == NS_URL_ERROR_CANCELLED => {
                        Err("Cancelled.".to_string())
                    }
                    Some(error) => Err(error.localizedDescription().to_string()),
                };
                let _ = self.ivars().tx.send(Event::Finished(result));
            }
        }

        unsafe impl NSURLSessionDataDelegate for StreamDelegate {
            #[unsafe(method(URLSession:dataTask:didReceiveData:))]
            fn did_receive_data(
                &self,
                _session: &NSURLSession,
                task: &NSURLSessionDataTask,
                data: &NSData,
            ) {
                self.send_status(task);
                if self.ivars().tx.send(Event::Data(data.to_vec())).is_err() {
                    // Nobody is listening any more.
                    task.cancel();
                }
            }
        }
    );

    impl StreamDelegate {
        fn new(tx: UnboundedSender<Event>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(Ivars {
                tx,
                sent_status: AtomicBool::new(false),
            });
            unsafe { msg_send![super(this), init] }
        }

        fn send_status(&self, task: &NSURLSessionTask) {
            if self.ivars().sent_status.swap(true, Ordering::Relaxed) {
                return;
            }
            let status = task
                .response()
                .and_then(|r| r.downcast::<NSHTTPURLResponse>().ok())
                .map_or(0, |r| r.statusCode() as u16);
            let _ = self.ivars().tx.send(Event::Status(status));
        }
    }

    pub fn send(request: HttpRequest, tx: UnboundedSender<Event>) -> Result<(), String> {
        let url = NSURL::URLWithString(&NSString::from_str(&request.url))
            .filter(|_| request.url.starts_with("http"))
            .ok_or_else(|| format!("“{}” isn’t a valid address.", request.url))?;
        let ns_request = NSMutableURLRequest::requestWithURL(&url);
        ns_request.setHTTPMethod(&NSString::from_str(request.method));
        for (name, value) in &request.headers {
            ns_request.setValue_forHTTPHeaderField(
                Some(&NSString::from_str(value)),
                &NSString::from_str(name),
            );
        }
        if let Some(body) = &request.body {
            ns_request.setHTTPBody(Some(&NSData::with_bytes(body)));
        }

        // Ephemeral: no cookies, cache or credentials written to disk.
        let config = NSURLSessionConfiguration::ephemeralSessionConfiguration();
        config.setTimeoutIntervalForRequest(IDLE_TIMEOUT_SECS);
        let delegate = StreamDelegate::new(tx);
        // SAFETY: a nil queue makes the session create its own serial queue,
        // and the delegate is Send + Sync.
        let session = unsafe {
            NSURLSession::sessionWithConfiguration_delegate_delegateQueue(
                &config,
                Some(ProtocolObject::from_ref(&*delegate)),
                None,
            )
        };
        let task = session.dataTaskWithRequest(&ns_request);
        task.resume();
        // The session keeps the delegate until its task is done, then lets go.
        session.finishTasksAndInvalidate();
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::{Event, HttpRequest, UnboundedSender};

    pub fn send(_request: HttpRequest, _tx: UnboundedSender<Event>) -> Result<(), String> {
        Err("Summaries are only available on macOS.".into())
    }
}
