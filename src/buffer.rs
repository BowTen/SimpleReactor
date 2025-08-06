use bytes::BytesMut;
use std::io::{self, Read, Write};

const INITIAL_SIZE: usize = 1024;
const PREPEND_SIZE: usize = 8;

pub struct Buffer {
    buffer: BytesMut,
    reader_index: usize,
    writer_index: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self::with_initial_size(INITIAL_SIZE)
    }

    pub fn with_initial_size(initial_size: usize) -> Self {
        let mut buffer = BytesMut::with_capacity(PREPEND_SIZE + initial_size);
        // 预留前置空间，用于在数据前添加长度等信息
        buffer.resize(PREPEND_SIZE + initial_size, 0);

        Self {
            buffer,
            reader_index: PREPEND_SIZE,
            writer_index: PREPEND_SIZE,
        }
    }

    pub fn read_tcp_stream(&mut self, stream: &mut mio::net::TcpStream) -> std::io::Result<usize> {
        let writable_start = self.writer_index;

        let mut bytes_read = stream.read(&mut self.buffer[writable_start..])?;
        if bytes_read > 0 {
            self.writer_index += bytes_read;
        }

        if self.writable_bytes() == 0 {
            let mut extra_buffer = [0; 65536]; // 64KB
            let extra_bytes_read = stream.read(&mut extra_buffer)?;
            if extra_bytes_read > 0 {
                self.append(&extra_buffer[..extra_bytes_read]);
                bytes_read += extra_bytes_read;
            }
        }

        Ok(bytes_read)
    }

    pub fn peek(&self, len: usize) -> Option<&[u8]> {
        if self.readable_bytes() >= len {
            Some(&self.buffer[self.reader_index..self.reader_index + len])
        } else {
            None
        }
    }

    pub fn retrieve(&mut self, len: usize) {
        if len <= self.readable_bytes() {
            self.reader_index += len;

            // 如果读取完了所有数据，重置索引以节省空间
            if self.reader_index == self.writer_index {
                self.reader_index = PREPEND_SIZE;
                self.writer_index = PREPEND_SIZE;
            }
        }
    }

    pub fn retrieve_all(&mut self) {
        self.reader_index = PREPEND_SIZE;
        self.writer_index = PREPEND_SIZE;
    }

    pub fn retrieve_as_string(&mut self, len: usize) -> Option<String> {
        if let Some(data) = self.peek(len) {
            let result = String::from_utf8_lossy(data).into_owned();
            self.retrieve(len);
            Some(result)
        } else {
            None
        }
    }

    pub fn retrieve_all_as_string(&mut self) -> String {
        let len = self.readable_bytes();
        self.retrieve_as_string(len).unwrap_or_default()
    }

    pub fn append(&mut self, data: &[u8]) {
        self.ensure_writable_bytes(data.len());

        let start = self.writer_index;
        let end = start + data.len();
        self.buffer[start..end].copy_from_slice(data);
        self.writer_index += data.len();
    }

    pub fn append_string(&mut self, s: &str) {
        self.append(s.as_bytes());
    }

    pub fn readable_bytes(&self) -> usize {
        self.writer_index - self.reader_index
    }

    pub fn writable_bytes(&self) -> usize {
        self.buffer.len() - self.writer_index
    }

    pub fn prepend_bytes(&self) -> usize {
        self.reader_index
    }

    // 确保有足够的可写空间
    fn ensure_writable_bytes(&mut self, len: usize) {
        if self.writable_bytes() < len {
            self.make_space(len);
        }
    }

    // 创建更多空间
    fn make_space(&mut self, len: usize) {
        let readable = self.readable_bytes();

        // 如果前面的空间加上后面的空间够用，就把数据向前移动
        if self.writable_bytes() + self.prepend_bytes() - PREPEND_SIZE >= len {
            // 移动数据到前面
            let reader_start = self.reader_index;

            for i in 0..readable {
                self.buffer[PREPEND_SIZE + i] = self.buffer[reader_start + i];
            }

            self.reader_index = PREPEND_SIZE;
            self.writer_index = PREPEND_SIZE + readable;
        } else {
            // 需要扩容
            let new_size = self.buffer.len() + len;
            self.buffer.resize(new_size, 0);
        }
    }

    // 查找特定字节序列
    pub fn find_bytes(&self, pattern: &[u8]) -> Option<usize> {
        if pattern.is_empty() || self.readable_bytes() < pattern.len() {
            return None;
        }

        let data = &self.buffer[self.reader_index..self.writer_index];

        for i in 0..=data.len() - pattern.len() {
            if &data[i..i + pattern.len()] == pattern {
                return Some(i);
            }
        }

        None
    }

    // 查找换行符
    pub fn find_crlf(&self) -> Option<usize> {
        self.find_bytes(b"\r\n")
    }

    pub fn find_lf(&self) -> Option<usize> {
        self.find_bytes(b"\n")
    }

    // 读取到指定分隔符为止
    pub fn read_until(&mut self, delimiter: &[u8]) -> Option<Vec<u8>> {
        if let Some(pos) = self.find_bytes(delimiter) {
            let result = self.buffer[self.reader_index..self.reader_index + pos].to_vec();
            self.retrieve(pos + delimiter.len()); // 包括分隔符
            Some(result)
        } else {
            None
        }
    }

    // 读取一行 (以\r\n或\n结尾)
    pub fn read_line(&mut self) -> Option<String> {
        if let Some(pos) = self.find_lf() {
            let line = if pos > 0 && self.buffer[self.reader_index + pos - 1] == b'\r' {
                String::from_utf8_lossy(
                    &self.buffer[self.reader_index..self.reader_index + pos - 1],
                )
                .into_owned()
            } else {
                String::from_utf8_lossy(&self.buffer[self.reader_index..self.reader_index + pos])
                    .into_owned()
            };
            self.retrieve(pos + 1); // 跳过 \n
            return Some(line);
        }

        None
    }

    // 获取内部数据的引用（用于直接写入socket等）
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.reader_index..self.writer_index]
    }

    // 获取可写区域的可变引用
    pub fn writable_slice(&mut self) -> &mut [u8] {
        let start = self.writer_index;
        &mut self.buffer[start..]
    }

    // 手动调整writer_index（在直接写入后调用）
    pub fn has_written(&mut self, len: usize) {
        self.writer_index += len;
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.append(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_buffer_creation() {
        let buffer = Buffer::new();
        assert_eq!(buffer.readable_bytes(), 0);
        assert!(buffer.writable_bytes() > 0);
    }

    #[test]
    fn test_append_and_read() {
        let mut buffer = Buffer::new();
        let data = b"Hello, World!";

        buffer.append(data);
        assert_eq!(buffer.readable_bytes(), data.len());

        let read_data = buffer.peek(data.len()).unwrap();
        assert_eq!(read_data, data);

        buffer.retrieve(5);
        assert_eq!(buffer.readable_bytes(), data.len() - 5);

        let remaining = buffer.peek(buffer.readable_bytes()).unwrap();
        assert_eq!(remaining, b", World!");
    }

    #[test]
    fn test_string_operations() {
        let mut buffer = Buffer::new();
        buffer.append_string("Hello, 世界!");

        let result = buffer.retrieve_all_as_string();
        assert_eq!(result, "Hello, 世界!");
        assert_eq!(buffer.readable_bytes(), 0);
    }

    #[test]
    fn test_line_reading() {
        let mut buffer = Buffer::new();
        buffer.append(b"Line 1\r\nLine 2\nLine 3\r\n");

        let line1 = buffer.read_line().unwrap();
        assert_eq!(line1, "Line 1");

        let line2 = buffer.read_line().unwrap();
        assert_eq!(line2, "Line 2");

        let line3 = buffer.read_line().unwrap();
        assert_eq!(line3, "Line 3");

        assert!(buffer.read_line().is_none());
    }

    #[test]
    fn test_find_bytes() {
        let mut buffer = Buffer::new();
        buffer.append(b"Hello, World! How are you?");

        let pos = buffer.find_bytes(b"World").unwrap();
        assert_eq!(pos, 7);

        let pos = buffer.find_bytes(b"xyz");
        assert!(pos.is_none());
    }

    #[test]
    fn test_read_until() {
        let mut buffer = Buffer::new();
        buffer.append(b"GET /path HTTP/1.1\r\nHost: example.com\r\n");

        let first_line = buffer.read_until(b"\r\n").unwrap();
        assert_eq!(first_line, b"GET /path HTTP/1.1");

        let second_line = buffer.read_until(b"\r\n").unwrap();
        assert_eq!(second_line, b"Host: example.com");
    }

    #[test]
    fn test_buffer_growth() {
        let mut buffer = Buffer::with_initial_size(10);
        let large_data = vec![b'x'; 2000];

        buffer.append(&large_data);
        assert_eq!(buffer.readable_bytes(), 2000);

        let read_back = buffer.peek(2000).unwrap();
        assert_eq!(read_back, &large_data[..]);
    }

    #[test]
    fn test_write_trait() {
        let mut buffer = Buffer::new();
        write!(buffer, "Hello, {}!", "World").unwrap();

        let result = buffer.retrieve_all_as_string();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_space_optimization() {
        let mut buffer = Buffer::new();

        // 添加数据
        buffer.append(b"0123456789");
        assert_eq!(buffer.readable_bytes(), 10);

        // 读取一部分
        buffer.retrieve(5);
        assert_eq!(buffer.readable_bytes(), 5);

        // 读取剩余部分，应该会重置索引
        buffer.retrieve(5);
        assert_eq!(buffer.readable_bytes(), 0);
        assert_eq!(buffer.reader_index, PREPEND_SIZE);
        assert_eq!(buffer.writer_index, PREPEND_SIZE);
    }

    #[test]
    fn test_retrieve_as_string() {
        let mut buffer = Buffer::new();
        buffer.append(b"Hello, World!");

        let hello = buffer.retrieve_as_string(5).unwrap();
        assert_eq!(hello, "Hello");
        assert_eq!(buffer.readable_bytes(), 8);

        let remaining = buffer.retrieve_as_string(100);
        assert!(remaining.is_none());

        let world = buffer.retrieve_as_string(8).unwrap();
        assert_eq!(world, ", World!");
        assert_eq!(buffer.readable_bytes(), 0);
    }
}
