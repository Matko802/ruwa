// uaaa-core.js - Enhanced Encryption Engine
/**
 * UAAA Encryption System
 * Uses LZ77 compression + XOR cipher + UA binary encoding
 * @param {string} t - Text to encrypt/decrypt
 * @param {string} k - Encryption key
 * @param {boolean} e - True to encrypt, false to decrypt
 * @returns {string} - Encrypted or decrypted text
 */
function _C(t, k, e) {
    const _s = '\x55\x41\x41\x41'; // Magic: "UAAA"

    function utf8Encode(str) {
        const bytes = [];
        for (let i = 0; i < str.length; i++) {
            let code = str.codePointAt(i);
            if (code > 0xffff) i++;
            if (code <= 0x7f) {
                bytes.push(code);
            } else if (code <= 0x7ff) {
                bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
            } else if (code <= 0xffff) {
                bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
            } else {
                bytes.push(0xf0 | (code >> 18), 0x80 | ((code >> 12) & 0x3f), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
            }
        }
        return bytes;
    }

    function utf8Decode(bytes) {
        let result = '';
        for (let i = 0; i < bytes.length; i++) {
            const b1 = bytes[i];
            if (b1 < 0x80) {
                result += String.fromCharCode(b1);
            } else if (b1 < 0xe0) {
                const b2 = bytes[++i] || 0;
                result += String.fromCodePoint(((b1 & 0x1f) << 6) | (b2 & 0x3f));
            } else if (b1 < 0xf0) {
                const b2 = bytes[++i] || 0;
                const b3 = bytes[++i] || 0;
                result += String.fromCodePoint(((b1 & 0x0f) << 12) | ((b2 & 0x3f) << 6) | (b3 & 0x3f));
            } else {
                const b2 = bytes[++i] || 0;
                const b3 = bytes[++i] || 0;
                const b4 = bytes[++i] || 0;
                result += String.fromCodePoint(((b1 & 0x07) << 18) | ((b2 & 0x3f) << 12) | ((b3 & 0x3f) << 6) | (b4 & 0x3f));
            }
        }
        return result;
    }

    // Input validation
    if (typeof t !== 'string') throw new TypeError('Input must be a string');
    if (typeof k !== 'string' || !k.length) throw new TypeError('Key must be non-empty string');
    if (!t.length && e) return _s; // Handle empty input for encryption

    if (e) {
        // === ENCRYPTION ===
        const utf8 = utf8Encode(t);
        const keyBytes = utf8Encode(k);
        let src = utf8.map((byte) => String.fromCharCode(byte)), dict = {}, _tc = [], phrase = src[0], code = 256;

        // LZ77-style compression
        for (let i = 1; i < src.length; i++) {
            let c = src[i], p = phrase + c;
            if (dict[p] !== undefined) {
                phrase = p;
            } else {
                _tc.push(phrase.length > 1 ? dict[phrase] : phrase.charCodeAt(0));
                if (code < 65535) dict[p] = code++;
                phrase = c;
            }
        }
        if (phrase) _tc.push(phrase.length > 1 ? dict[phrase] : phrase.charCodeAt(0));

        // Generate stronger IV using multiple random values
        let b = [Math.floor(Math.random() * 65536)], c = b[0];

        // XOR encryption with derived key bytes
        for (let i = 0; i < _tc.length; i++) {
            let keyByte = keyBytes[i % keyBytes.length] || 0;
            let w = _tc[i] ^ ((keyByte + c + i) % 65536);
            b.push(w);
            c = (c + w) % 65536;
        }

        // Binary to UA encoding
        let r = '';
        for (let v of b) {
            let p = v.toString(2).padStart(16, '0');
            for (let j of p) r += j === '0' ? '\x55' : '\x41';
        }
        return _s + r;
    } else {
        // === DECRYPTION ===
        let x = t.replace(/[^UA]/gi, '').toUpperCase();
        if (!x.startsWith(_s)) throw new Error('Invalid cipher signature');

        let q = x.substring(_s.length);

        // Remove padding
        let rm = q.length % 16;
        if (rm !== 0) q = q.substring(0, q.length - rm);
        if (!q) return '';

        // UA to binary decoding
        let b = [];
        for (let i = 0; i < q.length; i += 16) {
            let chunk = q.substring(i, i + 16);
            b.push(parseInt(chunk.replace(/U/g, '0').replace(/A/g, '1'), 2));
        }

        // XOR decryption using UTF-8 key bytes
        const keyBytes = utf8Encode(k);
        let c = b[0], _dc = [];
        for (let i = 1; i < b.length; i++) {
            let keyByte = keyBytes[(i - 1) % keyBytes.length] || 0;
            _dc.push(b[i] ^ ((keyByte + c + (i - 1)) % 65536));
            c = (c + b[i]) % 65536;
        }
        if (_dc.length === 0) return '';

        // LZ77 decompression
        let dict = {}, currChar = String.fromCharCode(_dc[0]), oldPhrase = currChar, out = [currChar], code = 256;
        for (let i = 1; i < _dc.length; i++) {
            let currCode = _dc[i], phrase;
            if (currCode < 256) {
                phrase = String.fromCharCode(currCode);
            } else {
                phrase = dict[currCode] ? dict[currCode] : (oldPhrase + currChar);
            }
            out.push(phrase);
            currChar = phrase.charAt(0);
            if (code < 65535) dict[code++] = oldPhrase + currChar;
            oldPhrase = phrase;
        }
        const decodedBytes = out.join('').split('').map((ch) => ch.charCodeAt(0));
        return utf8Decode(decodedBytes);
    }
}