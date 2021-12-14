const Header = require('postman-collection').Header;

const prefix = "127.0.0.1:8080";    // set your server addr
const secret = "secret";            // set your secret

const hexToBase64 = (hexstring) => {
    return btoa(hexstring.match(/\w{2}/g).map(function(a) {
        return String.fromCharCode(parseInt(a, 16));
    }).join(""));
}

const now = new Date().getTime();

const hmac = CryptoJS.algo.HMAC.create(CryptoJS.algo.SHA256, secret);
hmac.update(now.toString());
hmac.update(pm.request.url.toString().replace(prefix, ""));
hmac.update(pm.request.body != null ? pm.request.body.toString() : "");

const sign = hmac.finalize().toString();

pm.request.headers.add(new Header(`timestamp: ${now}`));
pm.request.headers.add(new Header(`sign: ${hexToBase64(sign)}`));
