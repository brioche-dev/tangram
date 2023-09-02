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
