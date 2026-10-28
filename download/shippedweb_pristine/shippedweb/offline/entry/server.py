import http.server
import socketserver
import json

PORT = 8081

class MyHandler(http.server.SimpleHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/graphql':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            print(f"Received GraphQL Request: {post_data.decode('utf-8')}")
            
            response = {
                "data": {
                    "Account": {
                        "myAccount": {
                            "id": "00000000000000000000000000000000",
                            "displayName": "LocalTester",
                            "email": "test@local.com"
                        }
                    }
                }
            }
            
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Headers', '*')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode('utf-8'))
        else:
            self.send_error(404)

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', '*')
        self.end_headers()

with socketserver.TCPServer(("", PORT), MyHandler) as httpd:
    print(f"Mock Server running at port {PORT}")
    httpd.serve_forever()
