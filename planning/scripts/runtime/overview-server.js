// MODE: PROD
// Minimal localhost HTTP server for the plan overview (node rung).
// Serves / (rendered HTML), /state.json (extractor output) and
// /sections/<id> (a fresh render's inner HTML for one section id).
// Delegates every render to the bash scripts; prints the bound port.
var http = require("http");
var child = require("child_process");
var fs = require("fs");
var os = require("os");
var path = require("path");

var planDir = process.argv[2];
var port = process.argv[3] ? parseInt(process.argv[3], 10) : 0;
var scriptDir = __dirname;
var stateScript = scriptDir + "/../overview-state.sh";
var renderScript = scriptDir + "/../render-plan-overview.sh";

function bash(script, args, cb) {
    child.execFile("bash", [script].concat(args), { maxBuffer: 32 * 1024 * 1024 },
        function (err, stdout, stderr) { cb(err, stdout, stderr); });
}

function sectionOf(html, id) {
    var m = html.match(new RegExp('<[^>]*id="' + id + '"[^>]*>[\\s\\S]*?(?=<[^>]*id="(?:identity-panel|step-details|tests-panel|coverage-panel|findings-panel|dep-graph|narr)"|</main>|</body>)'));
    return m ? m[0] : "";
}

// The renderer writes a file and prints its path; render to a temp and read
// it back so the response is exactly the artifact.
function render(cb) {
    var tmp = path.join(os.tmpdir(), "overview-serve-" + process.pid + ".html");
    child.execFile("bash", [renderScript, "--serve", planDir, "--out", tmp],
        { maxBuffer: 32 * 1024 * 1024 },
        function (err) {
            if (err) { return cb(null); }
            fs.readFile(tmp, "utf8", function (e2, data) {
                fs.unlink(tmp, function () {});
                cb(e2 ? null : data);
            });
        });
}

var server = http.createServer(function (req, res) {
    var urlPath = req.url.split("?")[0];
    if (urlPath === "/state.json" || urlPath === "/state") {
        bash(stateScript, [planDir], function (err, out) {
            if (err) { res.writeHead(500); return res.end(); }
            res.writeHead(200, { "Content-Type": "application/json; charset=utf-8" });
            res.end(out);
        });
    } else if (urlPath.indexOf("/sections/") === 0) {
        var id = urlPath.slice("/sections/".length).replace(/[^a-z-]/g, "");
        render(function (html) {
            var sec = html ? sectionOf(html, id) : null;
            if (!sec) { res.writeHead(404); return res.end(); }
            res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
            res.end(sec);
        });
    } else {
        render(function (html) {
            if (!html) { res.writeHead(500); return res.end(); }
            res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
            res.end(html);
        });
    }
});
server.listen(port, "127.0.0.1", function () {
    console.log(server.address().port);
});
