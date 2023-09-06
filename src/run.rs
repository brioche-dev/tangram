pub struct Run {
	build: Id,
	state: State,
}

pub enum State {
	Running { children: Vec<Rid> },
	Complete { log: Id, children: Vec<Rid> },
}

enum Event {
	Log(Vec<u8>),
	Child(Rid),
	Output(Output),
}

pub struct Run {
	id: Id,
	build: Id,
	state: State,
}
pub enum State {
	Running { children: Vec<Rid> },
	Complete { log: Id, children: Vec<Rid> },
}

pub struct Log(Vec<Event>);
pub enum Event {
	Stdout(Vec<u8>),
	Stderr(Vec<u8>),
	Child(Rid),
	Output(Option<Value>),
}
