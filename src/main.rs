use futures::task::AtomicWaker;
use slab::Slab;
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::Instant;

struct IoState {
    pub read_waker: AtomicWaker,
    pub write_waker: AtomicWaker,
}

struct TimerFuture {
    pub deadline: std::time::Instant,
    pub io_state: Arc<IoState>,
    pub state: Arc<State>,
}

impl TimerFuture {
    pub fn new(duration: std::time::Duration, io_state: Arc<IoState>, state: Arc<State>) -> Self {
        Self {
            io_state,
            deadline: std::time::Instant::now() + duration,
            state,
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            println!("[Future] TimerFuture completed");
            return Poll::Ready(());
        }

        println!("[Future] TimerFuture pending, registering waker");
        // For demonstration, we just return Pending
        self.io_state.read_waker.register(cx.waker());

        if let Some(slab_id) = *self.state.current_slab.read().unwrap() {
            println!(
                "[Future] IoState already registered in slab with id {}",
                slab_id
            );
        } else {
            println!("[Future] IoState not registered in slab, registering now");

            let slab_id = {
                let mut slab = self.state.slabs.write().unwrap();
                slab.insert(self.io_state.clone())
            };
            let state = self.state.clone();
            *state.current_slab.write().unwrap() = Some(slab_id);
            println!("[Future] Registered IoState in slab with id {}", slab_id);
        }
        Poll::Pending
    }
}

struct Task {
    pub future: Arc<Mutex<Pin<Box<dyn futures::Future<Output = ()> + Send + 'static>>>>,
    pub state: Arc<State>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        println!("[Waker] Waking task and adding to queue");
        let mut queue = self.state.task_queue.lock().unwrap();
        queue.push(self.clone());
    }
}

struct State {
    pub task_queue: Mutex<Vec<Arc<Task>>>,
    pub slabs: RwLock<Slab<Arc<IoState>>>,
    pub current_slab: Arc<RwLock<Option<usize>>>,
}

fn main() {
    println!("Hello, world!");

    let state = Arc::new(State {
        task_queue: Mutex::new(Vec::new()),
        slabs: RwLock::new(Slab::new()),
        current_slab: Arc::new(RwLock::new(None)),
    });

    let executor_state = state.clone();
    let reactor_state = state.clone();

    thread::spawn(move || {
        println!("Running the executor task");
        loop {
            let task = {
                let mut queue = executor_state.task_queue.lock().unwrap();
                queue.pop()
            };
            if let Some(task) = task {
                println!("Polling a task");
                let mut future = task.future.lock().unwrap();
                let waker = Waker::from(task.clone());
                let mut context = Context::from_waker(&waker);

                let _ = future.as_mut().poll(&mut context);
            }
        }
    });

    thread::spawn(move || {
        println!("Running the reactor task");
        // Wait on something and then wake the waker
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("[Reactor] Reactor waking tasks");
            let current_slab = {
                let slab_lock = reactor_state.current_slab.read().unwrap();
                *slab_lock
            };
            if current_slab.is_none() {
                println!("[Reactor] No current slab set, skipping");
                continue;
            }

            let current_slab = current_slab.unwrap();

            println!("[Reactor] Current slab id: {}", current_slab);

            let io_state = {
                let slabs = reactor_state.slabs.read().unwrap();
                slabs.get(current_slab).cloned()
            };

            if let Some(io_state) = io_state {
                println!("[Reactor] Got IoState from slab");
                println!("[Reactor] Waking read waker");
                io_state.read_waker.wake();
            }
        }
    });

    let task = Task {
        future: Arc::new(Mutex::new(Box::pin(TimerFuture::new(
            std::time::Duration::from_secs(5),
            Arc::new(IoState {
                read_waker: AtomicWaker::new(),
                write_waker: AtomicWaker::new(),
            }),
            state.clone(),
        )))),
        state: state.clone(),
    };
    state.task_queue.lock().unwrap().push(Arc::new(task));

    std::thread::sleep(std::time::Duration::from_secs(100));
}

// Executor
//  Creates a waker
//  1) Runs a loop, fetches from the queue, then polls the future.

// Reactor
//  1) Wakes up the future by adding to the queue
