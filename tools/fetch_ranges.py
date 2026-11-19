# Streams a large file to standard output in byte ranges, retrying each range, so one dropped
# connection does not end a long download. Usage: python3 tools/fetch_ranges.py URL > file
import sys, time, urllib.request

url = sys.argv[1]
CHUNK = 256 * 1024 * 1024
head = urllib.request.urlopen(urllib.request.Request(url, method="HEAD"), timeout=120)
size = int(head.headers["Content-Length"])
print(f"{size / 1e9:.1f} GB to fetch", file=sys.stderr, flush=True)
out = sys.stdout.buffer
pos = 0
while pos < size:
    end = min(pos + CHUNK, size) - 1
    for attempt in range(8):
        try:
            req = urllib.request.Request(url, headers={"Range": f"bytes={pos}-{end}"})
            with urllib.request.urlopen(req, timeout=300) as r:
                got = 0
                while True:
                    b = r.read(1 << 20)
                    if not b: break
                    out.write(b); got += len(b); pos += len(b)
            if pos > end: break
            raise IOError(f"short range: {got} bytes")
        except Exception as e:
            print(f"range at {pos}: {e}; retrying", file=sys.stderr, flush=True)
            time.sleep(10 * (attempt + 1))
    else:
        sys.exit(f"gave up at byte {pos}")
    print(f"{pos / 1e9:.1f} GB", file=sys.stderr, flush=True)
out.flush()
