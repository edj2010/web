#![allow(clippy::unused_io_amount)]

pub mod http;
use http::{Request, Response};

pub mod error;
use error::Result;

mod threadpool;
use threadpool::ThreadPool;

use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream},
    str::from_utf8,
};

const BUFFER_SIZE: usize = 65536;

#[derive(Debug)]
pub struct WebServer<RequestHandle: Fn(Request) -> Result<Response>> {
    listener: TcpListener,
    workers: ThreadPool,
    internal_error_page: Response,
    handler: RequestHandle,
}

impl<RequestHandle: Fn(Request) -> Result<Response>> WebServer<RequestHandle>
where
    RequestHandle: Sync + Send + Copy + 'static,
{
    pub fn new(
        addr: &str,
        worker_count: usize,
        handler: RequestHandle,
    ) -> Result<WebServer<RequestHandle>> {
        Ok(WebServer {
            listener: TcpListener::bind(addr)?,
            workers: ThreadPool::new(worker_count),
            internal_error_page: Response::empty_internal_server_error(),
            handler,
        })
    }

    fn handle_connection(
        handler: RequestHandle,
        mut stream: TcpStream,
        internal_error_page: Response,
    ) -> Result<()> {
        //TODO: Make read_to_end + remove buffer
        let mut buffer = [0; BUFFER_SIZE];
        stream.read(&mut buffer).unwrap();

        let request = Request::parse(from_utf8(&buffer)?)?;

        println!("\n-----\n{:?}\n-----\n", request);

        stream.write(&(handler)(request).unwrap_or(internal_error_page).to_raw())?;
        stream.flush()?;

        Ok(())
    }

    pub fn set_custom_internal_error_page(&mut self, response: Response) {
        self.internal_error_page = response;
    }

    pub fn launch(self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = self.handler.clone();
                    let internal_error_page = self.internal_error_page.clone();
                    self.workers.execute(move || {
                        Self::handle_connection(handler, stream, internal_error_page)
                            .unwrap_or_else(|e| println!("Error on handling request: {}", e))
                    })
                }
                Err(e) => println!("Error on connection attempt: {}", e),
            }
        }
    }
}

/*
No unit tests currently used

#[cfg(test)]
mod tests {
    use super::*;
}*/
