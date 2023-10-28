use crate::nfs::{
	server::Context,
	types::{FileHandle, NFS4ERR_BADHANDLE},
};

pub fn put(ctx: &mut Context, arg: FileHandle) {
	ctx.current_file_handle = Some(arg);
}

pub fn get(ctx: &Context) -> Result<FileHandle, i32> {
	ctx.current_file_handle.ok_or(NFS4ERR_BADHANDLE)
}

pub fn save(ctx: &mut Context) {
	ctx.saved_file_handle = ctx.current_file_handle;
}

pub fn restore(ctx: &mut Context) {
	ctx.current_file_handle = ctx.saved_file_handle.take();
}
