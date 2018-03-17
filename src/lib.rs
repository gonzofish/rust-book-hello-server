use std::sync::{Arc, mpsc, Mutex};
use std::thread;

trait FnBox {
  fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
  fn call_box(self: Box<F>) {
    (*self)()
  }
}

enum Message {
  NewJob(Job),
  Terminate,
}

type Job = Box<FnBox + Send + 'static>;

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Message>,
}

impl ThreadPool {
  pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, plain_receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(plain_receiver));
    let mut workers = Vec::with_capacity(size);

    for id in 0 .. size {
      workers.push(Worker::new(id, Arc::clone(&receiver)));
    }

    ThreadPool {
      workers,
      sender,
    }
  }

  pub fn execute<F>(&self, f: F)
    where F: FnOnce() + Send + 'static
  {
    let job = Box::new(f);

    self.sender
      .send(Message::NewJob(job))
      .unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    println!("Sending terminate message to all workers.");

    for _ in &mut self.workers {
      self.sender.send(Message::Terminate).unwrap();
    }

    println!("Shutting down works...");

    for worker in &mut self.workers {
      println!("\tShutting down worker {}", worker.id);

      if let Some(thread) = worker.thread.take() {
        thread.join().unwrap();
      }
    }
  }
}

struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
    let thread = thread::spawn(move || {
      loop {
        let message = receiver.lock() // get the Receiver<Message>
          .unwrap()
          .recv() // Receive the message
          .unwrap();

        match message {
          Message::NewJob(job) => {
            println!("Worker {} got a job! Executing...", id);
            job.call_box();
          },
          Message::Terminate => {
            println!("Worker {} was told to terminate...", id);
            break;
          }
        };
      };
    });

    Worker {
      id,
      thread: Some(thread),
    }
  }
}
