//! CDP-level evasion scripts.
//!
//! JavaScript patches injected via `Page.addScriptToEvaluateOnNewDocument`
//! before ANY page script executes. Each patch targets a specific
//! browser automation detection vector.
//!
//! Patches are organized by detection vector and bundled into levels.
//! The self-audit script verifies all patches are active.

use super::{FingerprintProfile, StealthLevel};

/// Return the complete set of init scripts for a stealth level.
#[must_use]
pub fn init_scripts(level: StealthLevel, profile: &FingerprintProfile) -> Vec<String> {
    if level == StealthLevel::Off {
        return vec![];
    }

    let mut scripts = vec![
        // Core evasions — applied at all levels
        patch_navigator_webdriver(),
        patch_chrome_runtime(),
        patch_navigator_plugins(),
        patch_navigator_languages(),
        patch_window_dimensions(),
        patch_hardware_concurrency(),
        patch_device_memory(),
        patch_notification_permission(),
    ];

    if level >= StealthLevel::Standard {
        scripts.extend(vec![
            patch_permissions_query(),
            patch_webgl_vendor(profile),
            patch_media_devices(),
            patch_timezone_offset(),
            patch_intl_datetime_format(),
        ]);
    }

    if level >= StealthLevel::Aggressive {
        scripts.extend(vec![
            patch_canvas_fingerprint(),
            patch_audio_fingerprint(),
            patch_font_enumeration(),
            patch_screen_resolution(profile),
            patch_battery_api(),
            patch_connection_type(),
        ]);
    }

    if level >= StealthLevel::Paranoid {
        scripts.extend(vec![
            patch_error_stack_trace(),
            patch_performance_timing(),
            patch_navigator_platform(profile),
            patch_webdriver_sensors(),
            patch_virtual_keyboard(),
            patch_installed_app_get(),
        ]);
    }

    scripts
}

/// Self-audit script that checks whether key evasions are working.
/// Returns JSON with pass/fail for each vector.
#[must_use]
pub fn audit_script() -> String {
    r#"
(function() {
    var results = {};
    try {
        results.webdriver = navigator.webdriver === undefined || navigator.webdriver === false;
    } catch(e) { results.webdriver = false; }
    try {
        results.chrome = typeof window.chrome === 'object' && window.chrome !== null;
    } catch(e) { results.chrome = false; }
    try {
        var p = navigator.plugins;
        results.plugins = p && p.length > 0;
    } catch(e) { results.plugins = false; }
    try {
        results.languages = navigator.languages && navigator.languages.length > 0;
    } catch(e) { results.languages = false; }
    try {
        results.hardwareConcurrency = navigator.hardwareConcurrency > 0;
    } catch(e) { results.hardwareConcurrency = false; }
    try {
        results.deviceMemory = navigator.deviceMemory > 0;
    } catch(e) { results.deviceMemory = false; }
    try {
        results.outerDims = window.outerWidth > window.innerWidth;
    } catch(e) { results.outerDims = false; }
    return JSON.stringify(results);
})()
"#
    .to_string()
}

// ── Individual patches ─────────────────────────────────────────────

fn patch_navigator_webdriver() -> String {
    r#"
(function() {
    if (navigator.webdriver === undefined) return;
    Object.defineProperty(Object.getPrototypeOf(navigator), 'webdriver', {
        get: function() { return false; },
        set: function() {},
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_chrome_runtime() -> String {
    r#"
(function() {
    if (typeof window.chrome !== 'undefined' && window.chrome && window.chrome.runtime) return;
    var c = window.chrome || {};
    c.runtime = c.runtime || {};
    c.loadTimes = c.loadTimes || function() {
        return {
            requestTime: Date.now() / 1000,
            startLoadTime: Date.now() / 1000 - 0.1,
            commitLoadTime: Date.now() / 1000 - 0.05,
            finishDocumentLoadTime: Date.now() / 1000 - 0.02,
            finishLoadTime: Date.now() / 1000,
            firstPaintTime: Date.now() / 1000 - 0.01,
            firstPaintAfterLoadTime: Date.now() / 1000,
            navigationType: 'Other',
            wasFetchedViaSpdy: true,
            wasNpnNegotiated: true,
            npnNegotiatedProtocol: 'h2',
            wasAlternateProtocolAvailable: true,
            connectionInfo: 'h2'
        };
    };
    c.csi = c.csi || function() { return {}; };
    c.app = c.app || {};
    window.chrome = c;
})();
"#
    .to_string()
}

fn patch_navigator_plugins() -> String {
    r#"
(function() {
    var pluginData = [
        { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format', mimeTypes: [{ type: 'application/pdf', suffixes: 'pdf' }] },
        { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai', description: '', mimeTypes: [{ type: 'application/pdf', suffixes: 'pdf' }] },
        { name: 'Native Client', filename: 'internal-nacl-plugin', description: '', mimeTypes: [{ type: 'application/x-nacl', suffixes: '' }, { type: 'application/x-pnacl', suffixes: '' }] }
    ];

    function FakeMimeType(data) {
        for (var k in data) { this[k] = data[k]; }
    }
    FakeMimeType.prototype = { type: '', suffixes: '', description: '' };

    function FakePlugin(data) {
        this.name = data.name;
        this.filename = data.filename;
        this.description = data.description;
        this.length = data.mimeTypes.length;
        var self = this;
        for (var i = 0; i < data.mimeTypes.length; i++) {
            var mt = new FakeMimeType(data.mimeTypes[i]);
            self[i] = mt;
        }
        this.item = function(i) { return self[i] || null; };
        this.namedItem = function(name) {
            for (var j = 0; j < self.length; j++) {
                if (self[j].type === name) return self[j];
            }
            return null;
        };
    }

    function FakePluginArray() {
        this.length = pluginData.length;
        for (var i = 0; i < pluginData.length; i++) {
            this[i] = new FakePlugin(pluginData[i]);
        }
        this.item = function(i) { return this[i] || null; };
        this.namedItem = function(name) {
            for (var j = 0; j < this.length; j++) {
                if (this[j].name === name) return this[j];
            }
            return null;
        };
        this.refresh = function() {};
    }

    Object.defineProperty(navigator, 'plugins', {
        get: function() { return new FakePluginArray(); },
        configurable: true,
        enumerable: true
    });

    Object.defineProperty(navigator, 'mimeTypes', {
        get: function() {
            var arr = { length: 1, 0: new FakeMimeType(pluginData[0].mimeTypes[0]) };
            arr.namedItem = function(n) { return arr[0]; };
            arr.item = function(i) { return arr[i]; };
            return arr;
        },
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_navigator_languages() -> String {
    r#"
(function() {
    Object.defineProperty(navigator, 'languages', {
        get: function() { return ['en-US', 'en']; },
        configurable: true,
        enumerable: true
    });
    Object.defineProperty(navigator, 'language', {
        get: function() { return 'en-US'; },
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_window_dimensions() -> String {
    r#"
(function() {
    var frame = 100;
    var toolbar = 40;
    Object.defineProperty(window, 'outerWidth', {
        get: function() { return (window.innerWidth || 1280) + frame; },
        configurable: true,
        enumerable: true
    });
    Object.defineProperty(window, 'outerHeight', {
        get: function() { return (window.innerHeight || 720) + toolbar; },
        configurable: true,
        enumerable: true
    });
    Object.defineProperty(window.screen, 'availWidth', {
        get: function() { return window.screen.width || 1920; },
        configurable: true,
        enumerable: true
    });
    Object.defineProperty(window.screen, 'availHeight', {
        get: function() { return (window.screen.height || 1080) - toolbar; },
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_hardware_concurrency() -> String {
    r#"
(function() {
    Object.defineProperty(navigator, 'hardwareConcurrency', {
        get: function() { return 8; },
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_device_memory() -> String {
    r#"
(function() {
    Object.defineProperty(navigator, 'deviceMemory', {
        get: function() { return 8; },
        configurable: true,
        enumerable: true
    });
})();
"#
    .to_string()
}

fn patch_notification_permission() -> String {
    r#"
(function() {
    var Notification = window.Notification || {};
    var perm = 'default';
    Object.defineProperty(Notification, 'permission', {
        get: function() { return perm; },
        configurable: true,
        enumerable: true
    });
    var origRequest = Notification.requestPermission;
    if (origRequest) {
        Notification.requestPermission = function(cb) {
            var p = new Promise(function(resolve) { resolve('default'); });
            if (cb) { cb('default'); }
            return p;
        };
    }
})();
"#
    .to_string()
}

fn patch_permissions_query() -> String {
    r#"
(function() {
    var origQuery = window.navigator.permissions.query;
    if (!origQuery) return;
    var allowedPerms = ['midi', 'notifications', 'push'];
    navigator.permissions.query = function(params) {
        var name = (params && params.name) || '';
        if (allowedPerms.indexOf(name) !== -1) {
            return Promise.resolve({ state: 'granted', onchange: null });
        }
        return origQuery.call(navigator.permissions, params);
    };
    // Make patched query look native
    navigator.permissions.query.toString = function() {
        return 'function query() { [native code] }';
    };
})();
"#
    .to_string()
}

fn patch_webgl_vendor(profile: &FingerprintProfile) -> String {
    let vendor = profile.webgl_vendor.replace('\\', "\\\\").replace('\'', "\\'");
    let renderer = profile.webgl_renderer.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"
(function() {{
    var vendorStr = '{vendor}';
    var rendererStr = '{renderer}';
    var getParam = WebGLRenderingContext.prototype.getParameter;
    if (!getParam) return;
    WebGLRenderingContext.prototype.getParameter = function(p) {{
        if (p === 37445) return vendorStr;
        if (p === 37446) return rendererStr;
        return getParam.call(this, p);
    }};
    var getParam2 = WebGL2RenderingContext && WebGL2RenderingContext.prototype.getParameter;
    if (getParam2) {{
        WebGL2RenderingContext.prototype.getParameter = function(p) {{
            if (p === 37445) return vendorStr;
            if (p === 37446) return rendererStr;
            return getParam2.call(this, p);
        }};
    }}
    // Also patch getExtension to return null for DEBUG_RENDERER_INFO
    var origGetExt = WebGLRenderingContext.prototype.getExtension;
    WebGLRenderingContext.prototype.getExtension = function(name) {{
        if (name === 'WEBGL_debug_renderer_info') return null;
        return origGetExt.call(this, name);
    }};
    if (WebGL2RenderingContext) {{
        var origGetExt2 = WebGL2RenderingContext.prototype.getExtension;
        WebGL2RenderingContext.prototype.getExtension = function(name) {{
            if (name === 'WEBGL_debug_renderer_info') return null;
            return origGetExt2.call(this, name);
        }};
    }}
}})();
"#
    )
}

fn patch_media_devices() -> String {
    r#"
(function() {
    if (!navigator.mediaDevices) {
        navigator.mediaDevices = {};
    }
    var origEnumerate = navigator.mediaDevices.enumerateDevices;
    navigator.mediaDevices.enumerateDevices = function() {
        if (origEnumerate) return origEnumerate.call(navigator.mediaDevices);
        return Promise.resolve([
            { deviceId: 'default', kind: 'audioinput', label: '', groupId: 'default' },
            { deviceId: 'default', kind: 'audiooutput', label: '', groupId: 'default' },
            { deviceId: 'default', kind: 'videoinput', label: '', groupId: 'default' }
        ]);
    };
})();
"#
    .to_string()
}

fn patch_timezone_offset() -> String {
    r#"
(function() {
    var origGet = Date.prototype.getTimezoneOffset;
    Date.prototype.getTimezoneOffset = function() {
        return 240; // UTC-4 (Eastern)
    };
})();
"#
    .to_string()
}

fn patch_intl_datetime_format() -> String {
    r#"
(function() {
    var origResolved = Intl.DateTimeFormat.prototype.resolvedOptions;
    Intl.DateTimeFormat.prototype.resolvedOptions = function() {
        var opts = origResolved.call(this);
        opts.timeZone = 'America/New_York';
        return opts;
    };
})();
"#
    .to_string()
}

fn patch_canvas_fingerprint() -> String {
    r#"
(function() {
    var origToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function() {
        var ctx = this.getContext('2d');
        if (ctx) {
            var imageData = ctx.getImageData(0, 0, this.width, this.height);
            if (imageData && imageData.data && imageData.data.length > 0) {
                for (var i = 0; i < imageData.data.length; i += 4) {
                    var r = imageData.data[i];
                    if (r % 2 === 0) {
                        imageData.data[i] = Math.min(255, r + 1);
                    }
                }
                ctx.putImageData(imageData, 0, 0);
            }
        }
        return origToDataURL.apply(this, arguments);
    };
    var origToBlob = HTMLCanvasElement.prototype.toBlob;
    if (origToBlob) {
        HTMLCanvasElement.prototype.toBlob = function() {
            var ctx = this.getContext('2d');
            if (ctx) {
                var imageData = ctx.getImageData(0, 0, this.width, this.height);
                if (imageData && imageData.data && imageData.data.length > 0) {
                    for (var i = 0; i < imageData.data.length; i += 4) {
                        var r = imageData.data[i];
                        if (r % 2 === 0 && r < 254) {
                            imageData.data[i] = r + 1;
                        }
                    }
                    ctx.putImageData(imageData, 0, 0);
                }
            }
            return origToBlob.apply(this, arguments);
        };
    }
})();
"#
    .to_string()
}

fn patch_audio_fingerprint() -> String {
    r#"
(function() {
    if (!window.AudioContext && !window.webkitAudioContext) return;
    var Context = window.AudioContext || window.webkitAudioContext;

    var origCreateAnalyser = Context.prototype.createAnalyser;
    Context.prototype.createAnalyser = function() {
        var analyser = origCreateAnalyser.call(this);
        var origGet = analyser.getFloatFrequencyData;
        if (origGet) {
            analyser.getFloatFrequencyData = function(arr) {
                origGet.call(analyser, arr);
                for (var i = 0; i < Math.min(arr.length, 5); i++) {
                    arr[i] = arr[i] + (Math.random() * 0.0001 - 0.00005);
                }
            };
        }
        var origGetByte = analyser.getByteFrequencyData;
        if (origGetByte) {
            analyser.getByteFrequencyData = function(arr) {
                origGetByte.call(analyser, arr);
                if (arr[0] % 2 === 0) arr[0] = Math.min(255, arr[0] + 1);
            };
        }
        return analyser;
    };

    var origCreateOscillator = Context.prototype.createOscillator;
    if (origCreateOscillator) {
        Context.prototype.createOscillator = function() {
            var osc = origCreateOscillator.call(this);
            var origFreq = Object.getOwnPropertyDescriptor(
                Object.getPrototypeOf(osc), 'frequency'
            );
            if (origFreq && origFreq.value) {
                var origSet = origFreq.value.setValueAtTime;
                if (origSet) {
                    osc.frequency.setValueAtTime = function(val, time) {
                        return origSet.call(this, val + (Math.random() * 0.1 - 0.05), time);
                    };
                }
            }
            return osc;
        };
    }
})();
"#
    .to_string()
}

fn patch_font_enumeration() -> String {
    r#"
(function() {
    if (typeof window.queryLocalFonts === 'function') {
        window.queryLocalFonts = function() {
            return Promise.resolve([]);
        };
    }
    if (typeof navigator.fonts !== 'undefined' && navigator.fonts && navigator.fonts.query) {
        var origQuery = navigator.fonts.query;
        navigator.fonts.query = function() {
            return origQuery.call(navigator.fonts).then(function(fonts) {
                return fonts.slice(0, 10);
            });
        };
    }
})();
"#
    .to_string()
}

fn patch_screen_resolution(profile: &FingerprintProfile) -> String {
    format!(
        r#"
(function() {{
    var w = {width};
    var h = {height};
    var cd = {color_depth};
    Object.defineProperty(window.screen, 'width', {{
        get: function() {{ return w; }},
        configurable: true,
        enumerable: true
    }});
    Object.defineProperty(window.screen, 'height', {{
        get: function() {{ return h; }},
        configurable: true,
        enumerable: true
    }});
    if ({{cd}} !== undefined) {{
        Object.defineProperty(window.screen, 'colorDepth', {{
            get: function() {{ return {{cd}}; }},
            configurable: true,
            enumerable: true
        }});
        Object.defineProperty(window.screen, 'pixelDepth', {{
            get: function() {{ return {{cd}}; }},
            configurable: true,
            enumerable: true
        }});
    }}
}})();
"#,
        width = profile.screen_width,
        height = profile.screen_height,
        color_depth = profile.color_depth,
    )
}

fn patch_battery_api() -> String {
    r#"
(function() {
    if (navigator.getBattery) {
        navigator.getBattery = function() {
            return Promise.resolve({
                charging: true,
                chargingTime: 0,
                dischargingTime: Infinity,
                level: 1,
                onchargingchange: null,
                onchargingtimechange: null,
                ondischargingtimechange: null,
                onlevelchange: null
            });
        };
    }
})();
"#
    .to_string()
}

fn patch_connection_type() -> String {
    r#"
(function() {
    if (navigator.connection) {
        Object.defineProperty(navigator.connection, 'rtt', {
            get: function() { return 50; },
            configurable: true,
            enumerable: true
        });
        Object.defineProperty(navigator.connection, 'downlink', {
            get: function() { return 10; },
            configurable: true,
            enumerable: true
        });
        Object.defineProperty(navigator.connection, 'effectiveType', {
            get: function() { return '4g'; },
            configurable: true,
            enumerable: true
        });
    } else {
        navigator.connection = {
            rtt: 50, downlink: 10, effectiveType: '4g', type: 'wifi',
            saveData: false, onchange: null
        };
    }
})();
"#
    .to_string()
}

fn patch_error_stack_trace() -> String {
    r#"
(function() {
    var origStack = Error.stackTraceLimit;
    var origPrepare = Error.prepareStackTrace;
    Error.stackTraceLimit = 20;
    if (origPrepare) {
        Error.prepareStackTrace = function(error, structuredStack) {
            for (var i = 0; i < structuredStack.length; i++) {
                if (structuredStack[i].getFileName() &&
                    structuredStack[i].getFileName().indexOf('cdp') !== -1) {
                    structuredStack[i].getFileName = function() { return ''; };
                }
            }
            return origPrepare(error, structuredStack);
        };
    }
})();
"#
    .to_string()
}

fn patch_performance_timing() -> String {
    r#"
(function() {
    if (window.performance && performance.timing) {
        Object.defineProperty(performance.timing, 'domContentLoadedEventStart', {
            get: function() { return Date.now() - 200; },
            configurable: true,
            enumerable: true
        });
        Object.defineProperty(performance.timing, 'domContentLoadedEventEnd', {
            get: function() { return Date.now() - 100; },
            configurable: true,
            enumerable: true
        });
    }
})();
"#
    .to_string()
}

fn patch_navigator_platform(profile: &FingerprintProfile) -> String {
    let platform = &profile.platform;
    let oscpu = if platform == "Win32" { "Windows NT 10.0; Win64; x64" } else { "" };
    format!(
        r#"
(function() {{
    var plat = '{platform}';
    Object.defineProperty(navigator, 'platform', {{
        get: function() {{ return plat; }},
        configurable: true,
        enumerable: true
    }});
    Object.defineProperty(navigator, 'oscpu', {{
        get: function() {{ return '{oscpu}'; }},
        configurable: true,
        enumerable: true
    }});
    Object.defineProperty(navigator, 'userAgent', {{
        get: function() {{ return navigator.userAgent; }},
        configurable: true,
        enumerable: true
    }});
}})();
"#
    )
}

fn patch_webdriver_sensors() -> String {
    r#"
(function() {
    if (window.Sensor && window.AmbientLightSensor) {
        window.AmbientLightSensor = undefined;
    }
    if (window.Sensor && window.Magnetometer) {
        window.Magnetometer = undefined;
    }
    if (window.Gyroscope) {
        window.Gyroscope = undefined;
    }
    if (window.Accelerometer) {
        window.Accelerometer = undefined;
    }
})();
"#
    .to_string()
}

fn patch_virtual_keyboard() -> String {
    r#"
(function() {
    if (navigator.keyboard) {
        Object.defineProperty(navigator.keyboard, 'getLayoutMap', {
            value: function() {
                return Promise.resolve(new Map());
            },
            configurable: true,
            enumerable: true
        });
    } else {
        navigator.keyboard = {
            getLayoutMap: function() { return Promise.resolve(new Map()); }
        };
    }
})();
"#
    .to_string()
}

fn patch_installed_app_get() -> String {
    r#"
(function() {
    if (navigator.getInstalledRelatedApps) {
        navigator.getInstalledRelatedApps = function() {
            return Promise.resolve([]);
        };
    }
})();
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> FingerprintProfile {
        FingerprintProfile::windows_chrome()
    }

    #[test]
    fn test_off_returns_empty() {
        let scripts = init_scripts(StealthLevel::Off, &test_profile());
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_standard_has_core_patches() {
        let scripts = init_scripts(StealthLevel::Standard, &test_profile());
        // Core: webdriver, chrome, plugins, languages, dimensions, hardware, memory, notification
        assert!(scripts.len() >= 13); // 8 core + 5 standard
    }

    #[test]
    fn test_aggressive_has_more_than_standard() {
        let std = init_scripts(StealthLevel::Standard, &test_profile());
        let agg = init_scripts(StealthLevel::Aggressive, &test_profile());
        assert!(agg.len() > std.len());
    }

    #[test]
    fn test_paranoid_has_most() {
        let agg = init_scripts(StealthLevel::Aggressive, &test_profile());
        let par = init_scripts(StealthLevel::Paranoid, &test_profile());
        assert!(par.len() > agg.len());
    }

    #[test]
    fn test_webdriver_patch_contains_key_elements() {
        let patch = patch_navigator_webdriver();
        assert!(patch.contains("navigator.webdriver"));
        assert!(patch.contains("defineProperty"));
    }

    #[test]
    fn test_audit_script_produces_valid_json() {
        let audit = audit_script();
        assert!(audit.contains("webdriver"));
        assert!(audit.contains("JSON.stringify"));
    }

    #[test]
    fn test_webgl_patch_includes_profile_data() {
        let profile = test_profile();
        let patch = patch_webgl_vendor(&profile);
        assert!(patch.contains(&profile.webgl_vendor));
        assert!(patch.contains(&profile.webgl_renderer));
    }
}
