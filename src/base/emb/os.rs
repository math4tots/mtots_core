pub(super) const SOURCE: &str = r######"
import _os
import _os.proc as osproc

name = _os::name
family = _os::family
arch = _os::arch

triple = [name, family, arch]

getcwd = _os::getcwd

"""
Main OS path separator (basically '\\' for windows and '/' for everyone else)
"""
sep = _os::sep

def* walk(root) {
    root = Path::new(root)

    if !root.is_dir() {
        """Maybe throw an exception"""
        return
    }

    stack = @[root]

    while stack {
        path = stack.pop()

        subpaths = path.list()
        dirs = subpaths.filter(def(p) = p.is_dir())
        files = subpaths.filter(def(p) = p.is_file())

        yield [path, dirs, files]

        stack.extend(dirs)
    }
}

INHERIT = :inherit
PIPE = :pipe
NULL = :null
UTF8 = :utf8

class Process {
    r###"
    Exposing Rust's 'Command'/'process' API
    "###

    [proc, encoding]

    static def __call(
            cmd,
            args=nil,
            stdin=nil,
            stdout=nil,
            stderr=nil,
            encoding=nil) = {
        stdin = stdin or INHERIT
        stdout = stdout or INHERIT
        stderr = stderr or INHERIT
        __malloc(
            Process,
            [osproc::spawn(cmd, args, stdin, stdout, stderr), encoding],
        )
    }

    def wait(self) = {
        [status, stdout, stderr] = osproc::wait(self.proc)
        if self.encoding is not nil {
            stdout = stdout.decode(self.encoding)
            stderr = stderr.decode(self.encoding)
        }
        [status, stdout, stderr]
    }
}

def run(*args, **kwargs) = {
    proc = Process(*args, **kwargs)
    proc.wait()
}
"######;
