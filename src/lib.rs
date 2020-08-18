pub mod instructions;
pub mod container;

pub(crate) mod parse;
pub(crate) mod gen;

// Workaround: cookie_factory doesn't mark File as seek for the purposes of BackToTheBuffer
pub struct CookieFile<'a>(pub &'a mut std::fs::File);

impl<'a> cookie_factory::Seek for CookieFile<'a> {}

impl<'a> std::io::Write for CookieFile<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    //#[inline]
    //fn is_write_vectored(&self) -> bool {
    //    self.0.is_write_vectored()
    //}

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl<'a> std::io::Seek for CookieFile<'a> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}