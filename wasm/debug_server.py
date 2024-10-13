from http.server import BaseHTTPRequestHandler, HTTPServer
import urllib.parse
import os

class RequestHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urllib.parse.urlparse(self.path)

        if self.path.startswith('/static/'):
            path = '.' + self.path
            if os.path.exists(path):
                self.send_response(200)
                self.send_header('Content-type', 'text/javascript')
                self.end_headers()

                with open(path, 'rb') as file:
                    self.wfile.write(file.read())
            else:
                self.send_response(404)
                self.send_header('Content-type', 'text/plain')
                self.end_headers()
                self.wfile.write(b'Not Found')
        else:
            path = './index.html'
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.send_header('Access-Control-Allow-Origin', 'null')
            self.end_headers()

            with open(path, 'rb') as file:
                self.wfile.write(file.read())

def run_server():
    server_address = ('0.0.0.0', 8000)
    httpd = HTTPServer(server_address, RequestHandler)
    print('Starting httpd on port 8000...')
    httpd.serve_forever()

run_server()
