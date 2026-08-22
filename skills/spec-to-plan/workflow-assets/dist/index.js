import { readFileSync as e, readdirSync as t } from "node:fs";
import "node:crypto";
import "node:fs/promises";
import "node:path";
import "node:url";
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/core.js
var n;
function r(e, t, n) {
	function r(n, r) {
		if (n._zod || Object.defineProperty(n, "_zod", {
			value: {
				def: r,
				constr: o,
				traits: /* @__PURE__ */ new Set()
			},
			enumerable: !1
		}), n._zod.traits.has(e)) return;
		n._zod.traits.add(e), t(n, r);
		let i = o.prototype, a = Object.keys(i);
		for (let e = 0; e < a.length; e++) {
			let t = a[e];
			t in n || (n[t] = i[t].bind(n));
		}
	}
	let i = n?.Parent ?? Object;
	class a extends i {}
	Object.defineProperty(a, "name", { value: e });
	function o(e) {
		var t;
		let i = n?.Parent ? new a() : this;
		r(i, e), (t = i._zod).deferred ?? (t.deferred = []);
		for (let e of i._zod.deferred) e();
		return i;
	}
	return Object.defineProperty(o, "init", { value: r }), Object.defineProperty(o, Symbol.hasInstance, { value: (t) => n?.Parent && t instanceof n.Parent ? !0 : t?._zod?.traits?.has(e) }), Object.defineProperty(o, "name", { value: e }), o;
}
var i = class extends Error {
	constructor() {
		super("Encountered Promise during synchronous parse. Use .parseAsync() instead.");
	}
}, a = class extends Error {
	constructor(e) {
		super(`Encountered unidirectional transform during encode: ${e}`), this.name = "ZodEncodeError";
	}
};
(n = globalThis).__zod_globalConfig ?? (n.__zod_globalConfig = {});
var o = globalThis.__zod_globalConfig;
function s(e) {
	return e && Object.assign(o, e), o;
}
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/util.js
function c(e) {
	let t = Object.values(e).filter((e) => typeof e == "number");
	return Object.entries(e).filter(([e, n]) => t.indexOf(+e) === -1).map(([e, t]) => t);
}
function l(e, t) {
	return typeof t == "bigint" ? t.toString() : t;
}
function u(e) {
	return { get value() {
		{
			let t = e();
			return Object.defineProperty(this, "value", { value: t }), t;
		}
		throw Error("cached value already set");
	} };
}
function d(e) {
	return e == null;
}
function f(e) {
	let t = +!!e.startsWith("^"), n = e.endsWith("$") ? e.length - 1 : e.length;
	return e.slice(t, n);
}
function p(e, t) {
	let n = e / t, r = Math.round(n), i = 2 ** -52 * Math.max(Math.abs(n), 1);
	return Math.abs(n - r) < i ? 0 : n - r;
}
var m = /* @__PURE__ */ Symbol("evaluating");
function h(e, t, n) {
	let r;
	Object.defineProperty(e, t, {
		get() {
			if (r !== m) return r === void 0 && (r = m, r = n()), r;
		},
		set(n) {
			Object.defineProperty(e, t, { value: n });
		},
		configurable: !0
	});
}
function g(e, t, n) {
	Object.defineProperty(e, t, {
		value: n,
		writable: !0,
		enumerable: !0,
		configurable: !0
	});
}
function _(...e) {
	let t = {};
	for (let n of e) Object.assign(t, Object.getOwnPropertyDescriptors(n));
	return Object.defineProperties({}, t);
}
function v(e) {
	return JSON.stringify(e);
}
function y(e) {
	return e.toLowerCase().trim().replace(/[^\w\s-]/g, "").replace(/[\s_-]+/g, "-").replace(/^-+|-+$/g, "");
}
var b = "captureStackTrace" in Error ? Error.captureStackTrace : (...e) => {};
function x(e) {
	return typeof e == "object" && !!e && !Array.isArray(e);
}
var S = /* @__PURE__ */ u(() => {
	if (o.jitless || typeof navigator < "u" && navigator?.userAgent?.includes("Cloudflare")) return !1;
	try {
		return Function(""), !0;
	} catch {
		return !1;
	}
});
function ee(e) {
	if (x(e) === !1) return !1;
	let t = e.constructor;
	if (t === void 0 || typeof t != "function") return !0;
	let n = t.prototype;
	return !(x(n) === !1 || Object.prototype.hasOwnProperty.call(n, "isPrototypeOf") === !1);
}
function te(e) {
	return ee(e) ? { ...e } : Array.isArray(e) ? [...e] : e instanceof Map ? new Map(e) : e instanceof Set ? new Set(e) : e;
}
var ne = /* @__PURE__ */ new Set([
	"string",
	"number",
	"symbol"
]);
function re(e) {
	return e.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
function C(e, t, n) {
	let r = new e._zod.constr(t ?? e._zod.def);
	return (!t || n?.parent) && (r._zod.parent = e), r;
}
function w(e) {
	let t = e;
	if (!t) return {};
	if (typeof t == "string") return { error: () => t };
	if (t?.message !== void 0) {
		if (t?.error !== void 0) throw Error("Cannot specify both `message` and `error` params");
		t.error = t.message;
	}
	return delete t.message, typeof t.error == "string" ? {
		...t,
		error: () => t.error
	} : t;
}
function ie(e) {
	return Object.keys(e).filter((t) => e[t]._zod.optin === "optional" && e[t]._zod.optout === "optional");
}
var ae = {
	safeint: [-(2 ** 53 - 1), 2 ** 53 - 1],
	int32: [-2147483648, 2147483647],
	uint32: [0, 4294967295],
	float32: [-34028234663852886e22, 34028234663852886e22],
	float64: [-Number.MAX_VALUE, Number.MAX_VALUE]
};
function oe(e, t) {
	let n = e._zod.def, r = n.checks;
	if (r && r.length > 0) throw Error(".pick() cannot be used on object schemas containing refinements");
	return C(e, _(e._zod.def, {
		get shape() {
			let e = {};
			for (let r in t) {
				if (!(r in n.shape)) throw Error(`Unrecognized key: "${r}"`);
				t[r] && (e[r] = n.shape[r]);
			}
			return g(this, "shape", e), e;
		},
		checks: []
	}));
}
function se(e, t) {
	let n = e._zod.def, r = n.checks;
	if (r && r.length > 0) throw Error(".omit() cannot be used on object schemas containing refinements");
	return C(e, _(e._zod.def, {
		get shape() {
			let r = { ...e._zod.def.shape };
			for (let e in t) {
				if (!(e in n.shape)) throw Error(`Unrecognized key: "${e}"`);
				t[e] && delete r[e];
			}
			return g(this, "shape", r), r;
		},
		checks: []
	}));
}
function ce(e, t) {
	if (!ee(t)) throw Error("Invalid input to extend: expected a plain object");
	let n = e._zod.def.checks;
	if (n && n.length > 0) {
		let n = e._zod.def.shape;
		for (let e in t) if (Object.getOwnPropertyDescriptor(n, e) !== void 0) throw Error("Cannot overwrite keys on object schemas containing refinements. Use `.safeExtend()` instead.");
	}
	return C(e, _(e._zod.def, { get shape() {
		let n = {
			...e._zod.def.shape,
			...t
		};
		return g(this, "shape", n), n;
	} }));
}
function le(e, t) {
	if (!ee(t)) throw Error("Invalid input to safeExtend: expected a plain object");
	return C(e, _(e._zod.def, { get shape() {
		let n = {
			...e._zod.def.shape,
			...t
		};
		return g(this, "shape", n), n;
	} }));
}
function ue(e, t) {
	if (e._zod.def.checks?.length) throw Error(".merge() cannot be used on object schemas containing refinements. Use .safeExtend() instead.");
	return C(e, _(e._zod.def, {
		get shape() {
			let n = {
				...e._zod.def.shape,
				...t._zod.def.shape
			};
			return g(this, "shape", n), n;
		},
		get catchall() {
			return t._zod.def.catchall;
		},
		checks: t._zod.def.checks ?? []
	}));
}
function de(e, t, n) {
	let r = t._zod.def.checks;
	if (r && r.length > 0) throw Error(".partial() cannot be used on object schemas containing refinements");
	return C(t, _(t._zod.def, {
		get shape() {
			let r = t._zod.def.shape, i = { ...r };
			if (n) for (let t in n) {
				if (!(t in r)) throw Error(`Unrecognized key: "${t}"`);
				n[t] && (i[t] = e ? new e({
					type: "optional",
					innerType: r[t]
				}) : r[t]);
			}
			else for (let t in r) i[t] = e ? new e({
				type: "optional",
				innerType: r[t]
			}) : r[t];
			return g(this, "shape", i), i;
		},
		checks: []
	}));
}
function fe(e, t, n) {
	return C(t, _(t._zod.def, { get shape() {
		let r = t._zod.def.shape, i = { ...r };
		if (n) for (let t in n) {
			if (!(t in i)) throw Error(`Unrecognized key: "${t}"`);
			n[t] && (i[t] = new e({
				type: "nonoptional",
				innerType: r[t]
			}));
		}
		else for (let t in r) i[t] = new e({
			type: "nonoptional",
			innerType: r[t]
		});
		return g(this, "shape", i), i;
	} }));
}
function pe(e, t = 0) {
	if (e.aborted === !0) return !0;
	for (let n = t; n < e.issues.length; n++) if (e.issues[n]?.continue !== !0) return !0;
	return !1;
}
function me(e, t = 0) {
	if (e.aborted === !0) return !0;
	for (let n = t; n < e.issues.length; n++) if (e.issues[n]?.continue === !1) return !0;
	return !1;
}
function he(e, t) {
	return t.map((t) => {
		var n;
		return (n = t).path ?? (n.path = []), t.path.unshift(e), t;
	});
}
function ge(e) {
	return typeof e == "string" ? e : e?.message;
}
function T(e, t, n) {
	let r = e.message ? e.message : ge(e.inst?._zod.def?.error?.(e)) ?? ge(t?.error?.(e)) ?? ge(n.customError?.(e)) ?? ge(n.localeError?.(e)) ?? "Invalid input", { inst: i, continue: a, input: o, ...s } = e;
	return s.path ??= [], s.message = r, t?.reportInput && (s.input = o), s;
}
function _e(e) {
	return Array.isArray(e) ? "array" : typeof e == "string" ? "string" : "unknown";
}
function ve(...e) {
	let [t, n, r] = e;
	return typeof t == "string" ? {
		message: t,
		code: "custom",
		input: n,
		inst: r
	} : { ...t };
}
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/errors.js
var ye = (e, t) => {
	e.name = "$ZodError", Object.defineProperty(e, "_zod", {
		value: e._zod,
		enumerable: !1
	}), Object.defineProperty(e, "issues", {
		value: t,
		enumerable: !1
	}), e.message = JSON.stringify(t, l, 2), Object.defineProperty(e, "toString", {
		value: () => e.message,
		enumerable: !1
	});
}, be = r("$ZodError", ye), xe = r("$ZodError", ye, { Parent: Error });
function Se(e, t = (e) => e.message) {
	let n = {}, r = [];
	for (let i of e.issues) i.path.length > 0 ? (n[i.path[0]] = n[i.path[0]] || [], n[i.path[0]].push(t(i))) : r.push(t(i));
	return {
		formErrors: r,
		fieldErrors: n
	};
}
function Ce(e, t = (e) => e.message) {
	let n = { _errors: [] }, r = (e, i = []) => {
		for (let a of e.issues) if (a.code === "invalid_union" && a.errors.length) a.errors.map((e) => r({ issues: e }, [...i, ...a.path]));
		else if (a.code === "invalid_key") r({ issues: a.issues }, [...i, ...a.path]);
		else if (a.code === "invalid_element") r({ issues: a.issues }, [...i, ...a.path]);
		else {
			let e = [...i, ...a.path];
			if (e.length === 0) n._errors.push(t(a));
			else {
				let r = n, i = 0;
				for (; i < e.length;) {
					let n = e[i];
					i === e.length - 1 ? (r[n] = r[n] || { _errors: [] }, r[n]._errors.push(t(a))) : r[n] = r[n] || { _errors: [] }, r = r[n], i++;
				}
			}
		}
	};
	return r(e), n;
}
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/parse.js
var we = (e) => (t, n, r, a) => {
	let o = r ? {
		...r,
		async: !1
	} : { async: !1 }, c = t._zod.run({
		value: n,
		issues: []
	}, o);
	if (c instanceof Promise) throw new i();
	if (c.issues.length) {
		let t = new (a?.Err ?? e)(c.issues.map((e) => T(e, o, s())));
		throw b(t, a?.callee), t;
	}
	return c.value;
}, Te = (e) => async (t, n, r, i) => {
	let a = r ? {
		...r,
		async: !0
	} : { async: !0 }, o = t._zod.run({
		value: n,
		issues: []
	}, a);
	if (o instanceof Promise && (o = await o), o.issues.length) {
		let t = new (i?.Err ?? e)(o.issues.map((e) => T(e, a, s())));
		throw b(t, i?.callee), t;
	}
	return o.value;
}, Ee = (e) => (t, n, r) => {
	let a = r ? {
		...r,
		async: !1
	} : { async: !1 }, o = t._zod.run({
		value: n,
		issues: []
	}, a);
	if (o instanceof Promise) throw new i();
	return o.issues.length ? {
		success: !1,
		error: new (e ?? be)(o.issues.map((e) => T(e, a, s())))
	} : {
		success: !0,
		data: o.value
	};
}, De = /* @__PURE__ */ Ee(xe), Oe = (e) => async (t, n, r) => {
	let i = r ? {
		...r,
		async: !0
	} : { async: !0 }, a = t._zod.run({
		value: n,
		issues: []
	}, i);
	return a instanceof Promise && (a = await a), a.issues.length ? {
		success: !1,
		error: new e(a.issues.map((e) => T(e, i, s())))
	} : {
		success: !0,
		data: a.value
	};
}, ke = /* @__PURE__ */ Oe(xe), Ae = (e) => (t, n, r) => {
	let i = r ? {
		...r,
		direction: "backward"
	} : { direction: "backward" };
	return we(e)(t, n, i);
}, je = (e) => (t, n, r) => we(e)(t, n, r), Me = (e) => async (t, n, r) => {
	let i = r ? {
		...r,
		direction: "backward"
	} : { direction: "backward" };
	return Te(e)(t, n, i);
}, Ne = (e) => async (t, n, r) => Te(e)(t, n, r), Pe = (e) => (t, n, r) => {
	let i = r ? {
		...r,
		direction: "backward"
	} : { direction: "backward" };
	return Ee(e)(t, n, i);
}, Fe = (e) => (t, n, r) => Ee(e)(t, n, r), Ie = (e) => async (t, n, r) => {
	let i = r ? {
		...r,
		direction: "backward"
	} : { direction: "backward" };
	return Oe(e)(t, n, i);
}, Le = (e) => async (t, n, r) => Oe(e)(t, n, r), Re = /^[cC][0-9a-z]{6,}$/, ze = /^[0-9a-z]+$/, Be = /^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/, Ve = /^[0-9a-vA-V]{20}$/, He = /^[A-Za-z0-9]{27}$/, Ue = /^[a-zA-Z0-9_-]{21}$/, We = /^P(?:(\d+W)|(?!.*W)(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+([.,]\d+)?S)?)?)$/, Ge = /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/, Ke = (e) => e ? RegExp(`^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-${e}[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$`) : /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$/, qe = /^(?!\.)(?!.*\.\.)([A-Za-z0-9_'+\-\.]*)[A-Za-z0-9_+-]@([A-Za-z0-9][A-Za-z0-9\-]*\.)+[A-Za-z]{2,}$/, Je = "^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$";
function Ye() {
	return new RegExp(Je, "u");
}
var Xe = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/, Ze = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:))$/, Qe = /^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/([0-9]|[1-2][0-9]|3[0-2])$/, $e = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|::|([0-9a-fA-F]{1,4})?::([0-9a-fA-F]{1,4}:?){0,6})\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/, et = /^$|^(?:[0-9a-zA-Z+/]{4})*(?:(?:[0-9a-zA-Z+/]{2}==)|(?:[0-9a-zA-Z+/]{3}=))?$/, tt = /^[A-Za-z0-9_-]*$/, nt = /^https?$/, rt = /^\+[1-9]\d{6,14}$/, it = "(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))", at = /* @__PURE__ */ RegExp(`^${it}$`);
function ot(e) {
	let t = "(?:[01]\\d|2[0-3]):[0-5]\\d";
	return typeof e.precision == "number" ? e.precision === -1 ? `${t}` : e.precision === 0 ? `${t}:[0-5]\\d` : `${t}:[0-5]\\d\\.\\d{${e.precision}}` : `${t}(?::[0-5]\\d(?:\\.\\d+)?)?`;
}
function st(e) {
	return RegExp(`^${ot(e)}$`);
}
function ct(e) {
	let t = ot({ precision: e.precision }), n = ["Z"];
	e.local && n.push(""), e.offset && n.push("([+-](?:[01]\\d|2[0-3]):[0-5]\\d)");
	let r = `${t}(?:${n.join("|")})`;
	return RegExp(`^${it}T(?:${r})$`);
}
var lt = (e) => {
	let t = e ? `[\\s\\S]{${e?.minimum ?? 0},${e?.maximum ?? ""}}` : "[\\s\\S]*";
	return RegExp(`^${t}$`);
}, ut = /^-?\d+$/, dt = /^-?\d+(?:\.\d+)?$/, ft = /^(?:true|false)$/i, pt = /^[^A-Z]*$/, mt = /^[^a-z]*$/, E = /* @__PURE__ */ r("$ZodCheck", (e, t) => {
	var n;
	e._zod ??= {}, e._zod.def = t, (n = e._zod).onattach ?? (n.onattach = []);
}), ht = {
	number: "number",
	bigint: "bigint",
	object: "date"
}, gt = /* @__PURE__ */ r("$ZodCheckLessThan", (e, t) => {
	E.init(e, t);
	let n = ht[typeof t.value];
	e._zod.onattach.push((e) => {
		let n = e._zod.bag, r = (t.inclusive ? n.maximum : n.exclusiveMaximum) ?? Infinity;
		t.value < r && (t.inclusive ? n.maximum = t.value : n.exclusiveMaximum = t.value);
	}), e._zod.check = (r) => {
		(t.inclusive ? r.value <= t.value : r.value < t.value) || r.issues.push({
			origin: n,
			code: "too_big",
			maximum: typeof t.value == "object" ? t.value.getTime() : t.value,
			input: r.value,
			inclusive: t.inclusive,
			inst: e,
			continue: !t.abort
		});
	};
}), _t = /* @__PURE__ */ r("$ZodCheckGreaterThan", (e, t) => {
	E.init(e, t);
	let n = ht[typeof t.value];
	e._zod.onattach.push((e) => {
		let n = e._zod.bag, r = (t.inclusive ? n.minimum : n.exclusiveMinimum) ?? -Infinity;
		t.value > r && (t.inclusive ? n.minimum = t.value : n.exclusiveMinimum = t.value);
	}), e._zod.check = (r) => {
		(t.inclusive ? r.value >= t.value : r.value > t.value) || r.issues.push({
			origin: n,
			code: "too_small",
			minimum: typeof t.value == "object" ? t.value.getTime() : t.value,
			input: r.value,
			inclusive: t.inclusive,
			inst: e,
			continue: !t.abort
		});
	};
}), vt = /* @__PURE__ */ r("$ZodCheckMultipleOf", (e, t) => {
	E.init(e, t), e._zod.onattach.push((e) => {
		var n;
		(n = e._zod.bag).multipleOf ?? (n.multipleOf = t.value);
	}), e._zod.check = (n) => {
		if (typeof n.value != typeof t.value) throw Error("Cannot mix number and bigint in multiple_of check.");
		(typeof n.value == "bigint" ? n.value % t.value === BigInt(0) : p(n.value, t.value) === 0) || n.issues.push({
			origin: typeof n.value,
			code: "not_multiple_of",
			divisor: t.value,
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), yt = /* @__PURE__ */ r("$ZodCheckNumberFormat", (e, t) => {
	E.init(e, t), t.format = t.format || "float64";
	let n = t.format?.includes("int"), r = n ? "int" : "number", [i, a] = ae[t.format];
	e._zod.onattach.push((e) => {
		let r = e._zod.bag;
		r.format = t.format, r.minimum = i, r.maximum = a, n && (r.pattern = ut);
	}), e._zod.check = (o) => {
		let s = o.value;
		if (n) {
			if (!Number.isInteger(s)) {
				o.issues.push({
					expected: r,
					format: t.format,
					code: "invalid_type",
					continue: !1,
					input: s,
					inst: e
				});
				return;
			}
			if (!Number.isSafeInteger(s)) {
				s > 0 ? o.issues.push({
					input: s,
					code: "too_big",
					maximum: 2 ** 53 - 1,
					note: "Integers must be within the safe integer range.",
					inst: e,
					origin: r,
					inclusive: !0,
					continue: !t.abort
				}) : o.issues.push({
					input: s,
					code: "too_small",
					minimum: -(2 ** 53 - 1),
					note: "Integers must be within the safe integer range.",
					inst: e,
					origin: r,
					inclusive: !0,
					continue: !t.abort
				});
				return;
			}
		}
		s < i && o.issues.push({
			origin: "number",
			input: s,
			code: "too_small",
			minimum: i,
			inclusive: !0,
			inst: e,
			continue: !t.abort
		}), s > a && o.issues.push({
			origin: "number",
			input: s,
			code: "too_big",
			maximum: a,
			inclusive: !0,
			inst: e,
			continue: !t.abort
		});
	};
}), bt = /* @__PURE__ */ r("$ZodCheckMaxLength", (e, t) => {
	var n;
	E.init(e, t), (n = e._zod.def).when ?? (n.when = (e) => {
		let t = e.value;
		return !d(t) && t.length !== void 0;
	}), e._zod.onattach.push((e) => {
		let n = e._zod.bag.maximum ?? Infinity;
		t.maximum < n && (e._zod.bag.maximum = t.maximum);
	}), e._zod.check = (n) => {
		let r = n.value;
		if (r.length <= t.maximum) return;
		let i = _e(r);
		n.issues.push({
			origin: i,
			code: "too_big",
			maximum: t.maximum,
			inclusive: !0,
			input: r,
			inst: e,
			continue: !t.abort
		});
	};
}), xt = /* @__PURE__ */ r("$ZodCheckMinLength", (e, t) => {
	var n;
	E.init(e, t), (n = e._zod.def).when ?? (n.when = (e) => {
		let t = e.value;
		return !d(t) && t.length !== void 0;
	}), e._zod.onattach.push((e) => {
		let n = e._zod.bag.minimum ?? -Infinity;
		t.minimum > n && (e._zod.bag.minimum = t.minimum);
	}), e._zod.check = (n) => {
		let r = n.value;
		if (r.length >= t.minimum) return;
		let i = _e(r);
		n.issues.push({
			origin: i,
			code: "too_small",
			minimum: t.minimum,
			inclusive: !0,
			input: r,
			inst: e,
			continue: !t.abort
		});
	};
}), St = /* @__PURE__ */ r("$ZodCheckLengthEquals", (e, t) => {
	var n;
	E.init(e, t), (n = e._zod.def).when ?? (n.when = (e) => {
		let t = e.value;
		return !d(t) && t.length !== void 0;
	}), e._zod.onattach.push((e) => {
		let n = e._zod.bag;
		n.minimum = t.length, n.maximum = t.length, n.length = t.length;
	}), e._zod.check = (n) => {
		let r = n.value, i = r.length;
		if (i === t.length) return;
		let a = _e(r), o = i > t.length;
		n.issues.push({
			origin: a,
			...o ? {
				code: "too_big",
				maximum: t.length
			} : {
				code: "too_small",
				minimum: t.length
			},
			inclusive: !0,
			exact: !0,
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), Ct = /* @__PURE__ */ r("$ZodCheckStringFormat", (e, t) => {
	var n, r;
	E.init(e, t), e._zod.onattach.push((e) => {
		let n = e._zod.bag;
		n.format = t.format, t.pattern && (n.patterns ??= /* @__PURE__ */ new Set(), n.patterns.add(t.pattern));
	}), t.pattern ? (n = e._zod).check ?? (n.check = (n) => {
		t.pattern.lastIndex = 0, !t.pattern.test(n.value) && n.issues.push({
			origin: "string",
			code: "invalid_format",
			format: t.format,
			input: n.value,
			...t.pattern ? { pattern: t.pattern.toString() } : {},
			inst: e,
			continue: !t.abort
		});
	}) : (r = e._zod).check ?? (r.check = () => {});
}), wt = /* @__PURE__ */ r("$ZodCheckRegex", (e, t) => {
	Ct.init(e, t), e._zod.check = (n) => {
		t.pattern.lastIndex = 0, !t.pattern.test(n.value) && n.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "regex",
			input: n.value,
			pattern: t.pattern.toString(),
			inst: e,
			continue: !t.abort
		});
	};
}), Tt = /* @__PURE__ */ r("$ZodCheckLowerCase", (e, t) => {
	t.pattern ??= pt, Ct.init(e, t);
}), Et = /* @__PURE__ */ r("$ZodCheckUpperCase", (e, t) => {
	t.pattern ??= mt, Ct.init(e, t);
}), Dt = /* @__PURE__ */ r("$ZodCheckIncludes", (e, t) => {
	E.init(e, t);
	let n = re(t.includes), r = new RegExp(typeof t.position == "number" ? `^.{${t.position}}${n}` : n);
	t.pattern = r, e._zod.onattach.push((e) => {
		let t = e._zod.bag;
		t.patterns ??= /* @__PURE__ */ new Set(), t.patterns.add(r);
	}), e._zod.check = (n) => {
		n.value.includes(t.includes, t.position) || n.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "includes",
			includes: t.includes,
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), Ot = /* @__PURE__ */ r("$ZodCheckStartsWith", (e, t) => {
	E.init(e, t);
	let n = RegExp(`^${re(t.prefix)}.*`);
	t.pattern ??= n, e._zod.onattach.push((e) => {
		let t = e._zod.bag;
		t.patterns ??= /* @__PURE__ */ new Set(), t.patterns.add(n);
	}), e._zod.check = (n) => {
		n.value.startsWith(t.prefix) || n.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "starts_with",
			prefix: t.prefix,
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), kt = /* @__PURE__ */ r("$ZodCheckEndsWith", (e, t) => {
	E.init(e, t);
	let n = RegExp(`.*${re(t.suffix)}$`);
	t.pattern ??= n, e._zod.onattach.push((e) => {
		let t = e._zod.bag;
		t.patterns ??= /* @__PURE__ */ new Set(), t.patterns.add(n);
	}), e._zod.check = (n) => {
		n.value.endsWith(t.suffix) || n.issues.push({
			origin: "string",
			code: "invalid_format",
			format: "ends_with",
			suffix: t.suffix,
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), At = /* @__PURE__ */ r("$ZodCheckOverwrite", (e, t) => {
	E.init(e, t), e._zod.check = (e) => {
		e.value = t.tx(e.value);
	};
}), jt = class {
	constructor(e = []) {
		this.content = [], this.indent = 0, this && (this.args = e);
	}
	indented(e) {
		this.indent += 1, e(this), --this.indent;
	}
	write(e) {
		if (typeof e == "function") {
			e(this, { execution: "sync" }), e(this, { execution: "async" });
			return;
		}
		let t = e.split("\n").filter((e) => e), n = Math.min(...t.map((e) => e.length - e.trimStart().length)), r = t.map((e) => e.slice(n)).map((e) => " ".repeat(this.indent * 2) + e);
		for (let e of r) this.content.push(e);
	}
	compile() {
		let e = Function, t = this?.args, n = [...(this?.content ?? [""]).map((e) => `  ${e}`)];
		return new e(...t, n.join("\n"));
	}
}, Mt = {
	major: 4,
	minor: 4,
	patch: 3
}, D = /* @__PURE__ */ r("$ZodType", (e, t) => {
	var n;
	e ??= {}, e._zod.def = t, e._zod.bag = e._zod.bag || {}, e._zod.version = Mt;
	let r = [...e._zod.def.checks ?? []];
	e._zod.traits.has("$ZodCheck") && r.unshift(e);
	for (let t of r) for (let n of t._zod.onattach) n(e);
	if (r.length === 0) (n = e._zod).deferred ?? (n.deferred = []), e._zod.deferred?.push(() => {
		e._zod.run = e._zod.parse;
	});
	else {
		let t = (e, t, n) => {
			let r = pe(e), a;
			for (let o of t) {
				if (o._zod.def.when) {
					if (me(e) || !o._zod.def.when(e)) continue;
				} else if (r) continue;
				let t = e.issues.length, s = o._zod.check(e);
				if (s instanceof Promise && n?.async === !1) throw new i();
				if (a || s instanceof Promise) a = (a ?? Promise.resolve()).then(async () => {
					await s, e.issues.length !== t && (r ||= pe(e, t));
				});
				else {
					if (e.issues.length === t) continue;
					r ||= pe(e, t);
				}
			}
			return a ? a.then(() => e) : e;
		}, n = (n, a, o) => {
			if (pe(n)) return n.aborted = !0, n;
			let s = t(a, r, o);
			if (s instanceof Promise) {
				if (o.async === !1) throw new i();
				return s.then((t) => e._zod.parse(t, o));
			}
			return e._zod.parse(s, o);
		};
		e._zod.run = (a, o) => {
			if (o.skipChecks) return e._zod.parse(a, o);
			if (o.direction === "backward") {
				let t = e._zod.parse({
					value: a.value,
					issues: []
				}, {
					...o,
					skipChecks: !0
				});
				return t instanceof Promise ? t.then((e) => n(e, a, o)) : n(t, a, o);
			}
			let s = e._zod.parse(a, o);
			if (s instanceof Promise) {
				if (o.async === !1) throw new i();
				return s.then((e) => t(e, r, o));
			}
			return t(s, r, o);
		};
	}
	h(e, "~standard", () => ({
		validate: (t) => {
			try {
				let n = De(e, t);
				return n.success ? { value: n.data } : { issues: n.error?.issues };
			} catch {
				return ke(e, t).then((e) => e.success ? { value: e.data } : { issues: e.error?.issues });
			}
		},
		vendor: "zod",
		version: 1
	}));
}), Nt = /* @__PURE__ */ r("$ZodString", (e, t) => {
	D.init(e, t), e._zod.pattern = [...e?._zod.bag?.patterns ?? []].pop() ?? lt(e._zod.bag), e._zod.parse = (n, r) => {
		if (t.coerce) try {
			n.value = String(n.value);
		} catch {}
		return typeof n.value == "string" || n.issues.push({
			expected: "string",
			code: "invalid_type",
			input: n.value,
			inst: e
		}), n;
	};
}), O = /* @__PURE__ */ r("$ZodStringFormat", (e, t) => {
	Ct.init(e, t), Nt.init(e, t);
}), Pt = /* @__PURE__ */ r("$ZodGUID", (e, t) => {
	t.pattern ??= Ge, O.init(e, t);
}), Ft = /* @__PURE__ */ r("$ZodUUID", (e, t) => {
	if (t.version) {
		let e = {
			v1: 1,
			v2: 2,
			v3: 3,
			v4: 4,
			v5: 5,
			v6: 6,
			v7: 7,
			v8: 8
		}[t.version];
		if (e === void 0) throw Error(`Invalid UUID version: "${t.version}"`);
		t.pattern ??= Ke(e);
	} else t.pattern ??= Ke();
	O.init(e, t);
}), It = /* @__PURE__ */ r("$ZodEmail", (e, t) => {
	t.pattern ??= qe, O.init(e, t);
}), Lt = /* @__PURE__ */ r("$ZodURL", (e, t) => {
	O.init(e, t), e._zod.check = (n) => {
		try {
			let r = n.value.trim();
			if (!t.normalize && t.protocol?.source === nt.source && !/^https?:\/\//i.test(r)) {
				n.issues.push({
					code: "invalid_format",
					format: "url",
					note: "Invalid URL format",
					input: n.value,
					inst: e,
					continue: !t.abort
				});
				return;
			}
			let i = new URL(r);
			t.hostname && (t.hostname.lastIndex = 0, t.hostname.test(i.hostname) || n.issues.push({
				code: "invalid_format",
				format: "url",
				note: "Invalid hostname",
				pattern: t.hostname.source,
				input: n.value,
				inst: e,
				continue: !t.abort
			})), t.protocol && (t.protocol.lastIndex = 0, t.protocol.test(i.protocol.endsWith(":") ? i.protocol.slice(0, -1) : i.protocol) || n.issues.push({
				code: "invalid_format",
				format: "url",
				note: "Invalid protocol",
				pattern: t.protocol.source,
				input: n.value,
				inst: e,
				continue: !t.abort
			})), t.normalize ? n.value = i.href : n.value = r;
			return;
		} catch {
			n.issues.push({
				code: "invalid_format",
				format: "url",
				input: n.value,
				inst: e,
				continue: !t.abort
			});
		}
	};
}), Rt = /* @__PURE__ */ r("$ZodEmoji", (e, t) => {
	t.pattern ??= Ye(), O.init(e, t);
}), zt = /* @__PURE__ */ r("$ZodNanoID", (e, t) => {
	t.pattern ??= Ue, O.init(e, t);
}), Bt = /* @__PURE__ */ r("$ZodCUID", (e, t) => {
	t.pattern ??= Re, O.init(e, t);
}), Vt = /* @__PURE__ */ r("$ZodCUID2", (e, t) => {
	t.pattern ??= ze, O.init(e, t);
}), Ht = /* @__PURE__ */ r("$ZodULID", (e, t) => {
	t.pattern ??= Be, O.init(e, t);
}), Ut = /* @__PURE__ */ r("$ZodXID", (e, t) => {
	t.pattern ??= Ve, O.init(e, t);
}), Wt = /* @__PURE__ */ r("$ZodKSUID", (e, t) => {
	t.pattern ??= He, O.init(e, t);
}), Gt = /* @__PURE__ */ r("$ZodISODateTime", (e, t) => {
	t.pattern ??= ct(t), O.init(e, t);
}), Kt = /* @__PURE__ */ r("$ZodISODate", (e, t) => {
	t.pattern ??= at, O.init(e, t);
}), qt = /* @__PURE__ */ r("$ZodISOTime", (e, t) => {
	t.pattern ??= st(t), O.init(e, t);
}), Jt = /* @__PURE__ */ r("$ZodISODuration", (e, t) => {
	t.pattern ??= We, O.init(e, t);
}), Yt = /* @__PURE__ */ r("$ZodIPv4", (e, t) => {
	t.pattern ??= Xe, O.init(e, t), e._zod.bag.format = "ipv4";
}), Xt = /* @__PURE__ */ r("$ZodIPv6", (e, t) => {
	t.pattern ??= Ze, O.init(e, t), e._zod.bag.format = "ipv6", e._zod.check = (n) => {
		try {
			new URL(`http://[${n.value}]`);
		} catch {
			n.issues.push({
				code: "invalid_format",
				format: "ipv6",
				input: n.value,
				inst: e,
				continue: !t.abort
			});
		}
	};
}), Zt = /* @__PURE__ */ r("$ZodCIDRv4", (e, t) => {
	t.pattern ??= Qe, O.init(e, t);
}), Qt = /* @__PURE__ */ r("$ZodCIDRv6", (e, t) => {
	t.pattern ??= $e, O.init(e, t), e._zod.check = (n) => {
		let r = n.value.split("/");
		try {
			if (r.length !== 2) throw Error();
			let [e, t] = r;
			if (!t) throw Error();
			let n = Number(t);
			if (`${n}` !== t || n < 0 || n > 128) throw Error();
			new URL(`http://[${e}]`);
		} catch {
			n.issues.push({
				code: "invalid_format",
				format: "cidrv6",
				input: n.value,
				inst: e,
				continue: !t.abort
			});
		}
	};
});
function $t(e) {
	if (e === "") return !0;
	if (/\s/.test(e) || e.length % 4 != 0) return !1;
	try {
		return atob(e), !0;
	} catch {
		return !1;
	}
}
var en = /* @__PURE__ */ r("$ZodBase64", (e, t) => {
	t.pattern ??= et, O.init(e, t), e._zod.bag.contentEncoding = "base64", e._zod.check = (n) => {
		$t(n.value) || n.issues.push({
			code: "invalid_format",
			format: "base64",
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
});
function tn(e) {
	if (!tt.test(e)) return !1;
	let t = e.replace(/[-_]/g, (e) => e === "-" ? "+" : "/");
	return $t(t.padEnd(Math.ceil(t.length / 4) * 4, "="));
}
var nn = /* @__PURE__ */ r("$ZodBase64URL", (e, t) => {
	t.pattern ??= tt, O.init(e, t), e._zod.bag.contentEncoding = "base64url", e._zod.check = (n) => {
		tn(n.value) || n.issues.push({
			code: "invalid_format",
			format: "base64url",
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), rn = /* @__PURE__ */ r("$ZodE164", (e, t) => {
	t.pattern ??= rt, O.init(e, t);
});
function an(e, t = null) {
	try {
		let n = e.split(".");
		if (n.length !== 3) return !1;
		let [r] = n;
		if (!r) return !1;
		let i = JSON.parse(atob(r));
		return !("typ" in i && i?.typ !== "JWT" || !i.alg || t && (!("alg" in i) || i.alg !== t));
	} catch {
		return !1;
	}
}
var on = /* @__PURE__ */ r("$ZodJWT", (e, t) => {
	O.init(e, t), e._zod.check = (n) => {
		an(n.value, t.alg) || n.issues.push({
			code: "invalid_format",
			format: "jwt",
			input: n.value,
			inst: e,
			continue: !t.abort
		});
	};
}), sn = /* @__PURE__ */ r("$ZodNumber", (e, t) => {
	D.init(e, t), e._zod.pattern = e._zod.bag.pattern ?? dt, e._zod.parse = (n, r) => {
		if (t.coerce) try {
			n.value = Number(n.value);
		} catch {}
		let i = n.value;
		if (typeof i == "number" && !Number.isNaN(i) && Number.isFinite(i)) return n;
		let a = typeof i == "number" ? Number.isNaN(i) ? "NaN" : Number.isFinite(i) ? void 0 : "Infinity" : void 0;
		return n.issues.push({
			expected: "number",
			code: "invalid_type",
			input: i,
			inst: e,
			...a ? { received: a } : {}
		}), n;
	};
}), cn = /* @__PURE__ */ r("$ZodNumberFormat", (e, t) => {
	yt.init(e, t), sn.init(e, t);
}), ln = /* @__PURE__ */ r("$ZodBoolean", (e, t) => {
	D.init(e, t), e._zod.pattern = ft, e._zod.parse = (n, r) => {
		if (t.coerce) try {
			n.value = !!n.value;
		} catch {}
		let i = n.value;
		return typeof i == "boolean" || n.issues.push({
			expected: "boolean",
			code: "invalid_type",
			input: i,
			inst: e
		}), n;
	};
}), un = /* @__PURE__ */ r("$ZodUnknown", (e, t) => {
	D.init(e, t), e._zod.parse = (e) => e;
}), dn = /* @__PURE__ */ r("$ZodNever", (e, t) => {
	D.init(e, t), e._zod.parse = (t, n) => (t.issues.push({
		expected: "never",
		code: "invalid_type",
		input: t.value,
		inst: e
	}), t);
});
function fn(e, t, n) {
	e.issues.length && t.issues.push(...he(n, e.issues)), t.value[n] = e.value;
}
var pn = /* @__PURE__ */ r("$ZodArray", (e, t) => {
	D.init(e, t), e._zod.parse = (n, r) => {
		let i = n.value;
		if (!Array.isArray(i)) return n.issues.push({
			expected: "array",
			code: "invalid_type",
			input: i,
			inst: e
		}), n;
		n.value = Array(i.length);
		let a = [];
		for (let e = 0; e < i.length; e++) {
			let o = i[e], s = t.element._zod.run({
				value: o,
				issues: []
			}, r);
			s instanceof Promise ? a.push(s.then((t) => fn(t, n, e))) : fn(s, n, e);
		}
		return a.length ? Promise.all(a).then(() => n) : n;
	};
});
function mn(e, t, n, r, i, a) {
	let o = n in r;
	if (e.issues.length) {
		if (i && a && !o) return;
		t.issues.push(...he(n, e.issues));
	}
	if (!o && !i) {
		e.issues.length || t.issues.push({
			code: "invalid_type",
			expected: "nonoptional",
			input: void 0,
			path: [n]
		});
		return;
	}
	e.value === void 0 ? o && (t.value[n] = void 0) : t.value[n] = e.value;
}
function hn(e) {
	let t = Object.keys(e.shape);
	for (let n of t) if (!e.shape?.[n]?._zod?.traits?.has("$ZodType")) throw Error(`Invalid element at key "${n}": expected a Zod schema`);
	let n = ie(e.shape);
	return {
		...e,
		keys: t,
		keySet: new Set(t),
		numKeys: t.length,
		optionalKeys: new Set(n)
	};
}
function gn(e, t, n, r, i, a) {
	let o = [], s = i.keySet, c = i.catchall._zod, l = c.def.type, u = c.optin === "optional", d = c.optout === "optional";
	for (let i in t) {
		if (i === "__proto__" || s.has(i)) continue;
		if (l === "never") {
			o.push(i);
			continue;
		}
		let a = c.run({
			value: t[i],
			issues: []
		}, r);
		a instanceof Promise ? e.push(a.then((e) => mn(e, n, i, t, u, d))) : mn(a, n, i, t, u, d);
	}
	return o.length && n.issues.push({
		code: "unrecognized_keys",
		keys: o,
		input: t,
		inst: a
	}), e.length ? Promise.all(e).then(() => n) : n;
}
var _n = /* @__PURE__ */ r("$ZodObject", (e, t) => {
	if (D.init(e, t), !Object.getOwnPropertyDescriptor(t, "shape")?.get) {
		let e = t.shape;
		Object.defineProperty(t, "shape", { get: () => {
			let n = { ...e };
			return Object.defineProperty(t, "shape", { value: n }), n;
		} });
	}
	let n = u(() => hn(t));
	h(e._zod, "propValues", () => {
		let e = t.shape, n = {};
		for (let t in e) {
			let r = e[t]._zod;
			if (r.values) {
				n[t] ?? (n[t] = /* @__PURE__ */ new Set());
				for (let e of r.values) n[t].add(e);
			}
		}
		return n;
	});
	let r = x, i = t.catchall, a;
	e._zod.parse = (t, o) => {
		a ??= n.value;
		let s = t.value;
		if (!r(s)) return t.issues.push({
			expected: "object",
			code: "invalid_type",
			input: s,
			inst: e
		}), t;
		t.value = {};
		let c = [], l = a.shape;
		for (let e of a.keys) {
			let n = l[e], r = n._zod.optin === "optional", i = n._zod.optout === "optional", a = n._zod.run({
				value: s[e],
				issues: []
			}, o);
			a instanceof Promise ? c.push(a.then((n) => mn(n, t, e, s, r, i))) : mn(a, t, e, s, r, i);
		}
		return i ? gn(c, s, t, o, n.value, e) : c.length ? Promise.all(c).then(() => t) : t;
	};
}), vn = /* @__PURE__ */ r("$ZodObjectJIT", (e, t) => {
	_n.init(e, t);
	let n = e._zod.parse, r = u(() => hn(t)), i = (e) => {
		let t = new jt([
			"shape",
			"payload",
			"ctx"
		]), n = r.value, i = (e) => {
			let t = v(e);
			return `shape[${t}]._zod.run({ value: input[${t}], issues: [] }, ctx)`;
		};
		t.write("const input = payload.value;");
		let a = Object.create(null), o = 0;
		for (let e of n.keys) a[e] = `key_${o++}`;
		t.write("const newResult = {};");
		for (let r of n.keys) {
			let n = a[r], o = v(r), s = e[r], c = s?._zod?.optin === "optional", l = s?._zod?.optout === "optional";
			t.write(`const ${n} = ${i(r)};`), c && l ? t.write(`
        if (${n}.issues.length) {
          if (${o} in input) {
            payload.issues = payload.issues.concat(${n}.issues.map(iss => ({
              ...iss,
              path: iss.path ? [${o}, ...iss.path] : [${o}]
            })));
          }
        }
        
        if (${n}.value === undefined) {
          if (${o} in input) {
            newResult[${o}] = undefined;
          }
        } else {
          newResult[${o}] = ${n}.value;
        }
        
      `) : c ? t.write(`
        if (${n}.issues.length) {
          payload.issues = payload.issues.concat(${n}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${o}, ...iss.path] : [${o}]
          })));
        }
        
        if (${n}.value === undefined) {
          if (${o} in input) {
            newResult[${o}] = undefined;
          }
        } else {
          newResult[${o}] = ${n}.value;
        }
        
      `) : t.write(`
        const ${n}_present = ${o} in input;
        if (${n}.issues.length) {
          payload.issues = payload.issues.concat(${n}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${o}, ...iss.path] : [${o}]
          })));
        }
        if (!${n}_present && !${n}.issues.length) {
          payload.issues.push({
            code: "invalid_type",
            expected: "nonoptional",
            input: undefined,
            path: [${o}]
          });
        }

        if (${n}_present) {
          if (${n}.value === undefined) {
            newResult[${o}] = undefined;
          } else {
            newResult[${o}] = ${n}.value;
          }
        }

      `);
		}
		t.write("payload.value = newResult;"), t.write("return payload;");
		let s = t.compile();
		return (t, n) => s(e, t, n);
	}, a, s = x, c = !o.jitless, l = c && S.value, d = t.catchall, f;
	e._zod.parse = (o, u) => {
		f ??= r.value;
		let p = o.value;
		return s(p) ? c && l && u?.async === !1 && u.jitless !== !0 ? (a ||= i(t.shape), o = a(o, u), d ? gn([], p, o, u, f, e) : o) : n(o, u) : (o.issues.push({
			expected: "object",
			code: "invalid_type",
			input: p,
			inst: e
		}), o);
	};
});
function yn(e, t, n, r) {
	for (let n of e) if (n.issues.length === 0) return t.value = n.value, t;
	let i = e.filter((e) => !pe(e));
	return i.length === 1 ? (t.value = i[0].value, i[0]) : (t.issues.push({
		code: "invalid_union",
		input: t.value,
		inst: n,
		errors: e.map((e) => e.issues.map((e) => T(e, r, s())))
	}), t);
}
var bn = /* @__PURE__ */ r("$ZodUnion", (e, t) => {
	D.init(e, t), h(e._zod, "optin", () => t.options.some((e) => e._zod.optin === "optional") ? "optional" : void 0), h(e._zod, "optout", () => t.options.some((e) => e._zod.optout === "optional") ? "optional" : void 0), h(e._zod, "values", () => {
		if (t.options.every((e) => e._zod.values)) return new Set(t.options.flatMap((e) => Array.from(e._zod.values)));
	}), h(e._zod, "pattern", () => {
		if (t.options.every((e) => e._zod.pattern)) {
			let e = t.options.map((e) => e._zod.pattern);
			return RegExp(`^(${e.map((e) => f(e.source)).join("|")})$`);
		}
	});
	let n = t.options.length === 1 ? t.options[0]._zod.run : null;
	e._zod.parse = (r, i) => {
		if (n) return n(r, i);
		let a = !1, o = [];
		for (let e of t.options) {
			let t = e._zod.run({
				value: r.value,
				issues: []
			}, i);
			if (t instanceof Promise) o.push(t), a = !0;
			else {
				if (t.issues.length === 0) return t;
				o.push(t);
			}
		}
		return a ? Promise.all(o).then((t) => yn(t, r, e, i)) : yn(o, r, e, i);
	};
}), xn = /* @__PURE__ */ r("$ZodDiscriminatedUnion", (e, t) => {
	t.inclusive = !1, bn.init(e, t);
	let n = e._zod.parse;
	h(e._zod, "propValues", () => {
		let e = {};
		for (let n of t.options) {
			let r = n._zod.propValues;
			if (!r || Object.keys(r).length === 0) throw Error(`Invalid discriminated union option at index "${t.options.indexOf(n)}"`);
			for (let [t, n] of Object.entries(r)) {
				e[t] || (e[t] = /* @__PURE__ */ new Set());
				for (let r of n) e[t].add(r);
			}
		}
		return e;
	});
	let r = u(() => {
		let e = t.options, n = /* @__PURE__ */ new Map();
		for (let r of e) {
			let e = r._zod.propValues?.[t.discriminator];
			if (!e || e.size === 0) throw Error(`Invalid discriminated union option at index "${t.options.indexOf(r)}"`);
			for (let t of e) {
				if (n.has(t)) throw Error(`Duplicate discriminator value "${String(t)}"`);
				n.set(t, r);
			}
		}
		return n;
	});
	e._zod.parse = (i, a) => {
		let o = i.value;
		if (!x(o)) return i.issues.push({
			code: "invalid_type",
			expected: "object",
			input: o,
			inst: e
		}), i;
		let s = r.value.get(o?.[t.discriminator]);
		return s ? s._zod.run(i, a) : t.unionFallback || a.direction === "backward" ? n(i, a) : (i.issues.push({
			code: "invalid_union",
			errors: [],
			note: "No matching discriminator",
			discriminator: t.discriminator,
			options: Array.from(r.value.keys()),
			input: o,
			path: [t.discriminator],
			inst: e
		}), i);
	};
}), Sn = /* @__PURE__ */ r("$ZodIntersection", (e, t) => {
	D.init(e, t), e._zod.parse = (e, n) => {
		let r = e.value, i = t.left._zod.run({
			value: r,
			issues: []
		}, n), a = t.right._zod.run({
			value: r,
			issues: []
		}, n);
		return i instanceof Promise || a instanceof Promise ? Promise.all([i, a]).then(([t, n]) => wn(e, t, n)) : wn(e, i, a);
	};
});
function Cn(e, t) {
	if (e === t || e instanceof Date && t instanceof Date && +e == +t) return {
		valid: !0,
		data: e
	};
	if (ee(e) && ee(t)) {
		let n = Object.keys(t), r = Object.keys(e).filter((e) => n.indexOf(e) !== -1), i = {
			...e,
			...t
		};
		for (let n of r) {
			let r = Cn(e[n], t[n]);
			if (!r.valid) return {
				valid: !1,
				mergeErrorPath: [n, ...r.mergeErrorPath]
			};
			i[n] = r.data;
		}
		return {
			valid: !0,
			data: i
		};
	}
	if (Array.isArray(e) && Array.isArray(t)) {
		if (e.length !== t.length) return {
			valid: !1,
			mergeErrorPath: []
		};
		let n = [];
		for (let r = 0; r < e.length; r++) {
			let i = e[r], a = t[r], o = Cn(i, a);
			if (!o.valid) return {
				valid: !1,
				mergeErrorPath: [r, ...o.mergeErrorPath]
			};
			n.push(o.data);
		}
		return {
			valid: !0,
			data: n
		};
	}
	return {
		valid: !1,
		mergeErrorPath: []
	};
}
function wn(e, t, n) {
	let r = /* @__PURE__ */ new Map(), i;
	for (let n of t.issues) if (n.code === "unrecognized_keys") {
		i ??= n;
		for (let e of n.keys) r.has(e) || r.set(e, {}), r.get(e).l = !0;
	} else e.issues.push(n);
	for (let t of n.issues) if (t.code === "unrecognized_keys") for (let e of t.keys) r.has(e) || r.set(e, {}), r.get(e).r = !0;
	else e.issues.push(t);
	let a = [...r].filter(([, e]) => e.l && e.r).map(([e]) => e);
	if (a.length && i && e.issues.push({
		...i,
		keys: a
	}), pe(e)) return e;
	let o = Cn(t.value, n.value);
	if (!o.valid) throw Error(`Unmergable intersection. Error path: ${JSON.stringify(o.mergeErrorPath)}`);
	return e.value = o.data, e;
}
var Tn = /* @__PURE__ */ r("$ZodRecord", (e, t) => {
	D.init(e, t), e._zod.parse = (n, r) => {
		let i = n.value;
		if (!ee(i)) return n.issues.push({
			expected: "record",
			code: "invalid_type",
			input: i,
			inst: e
		}), n;
		let a = [], o = t.keyType._zod.values;
		if (o) {
			n.value = {};
			let c = /* @__PURE__ */ new Set();
			for (let l of o) if (typeof l == "string" || typeof l == "number" || typeof l == "symbol") {
				c.add(typeof l == "number" ? l.toString() : l);
				let o = t.keyType._zod.run({
					value: l,
					issues: []
				}, r);
				if (o instanceof Promise) throw Error("Async schemas not supported in object keys currently");
				if (o.issues.length) {
					n.issues.push({
						code: "invalid_key",
						origin: "record",
						issues: o.issues.map((e) => T(e, r, s())),
						input: l,
						path: [l],
						inst: e
					});
					continue;
				}
				let u = o.value, d = t.valueType._zod.run({
					value: i[l],
					issues: []
				}, r);
				d instanceof Promise ? a.push(d.then((e) => {
					e.issues.length && n.issues.push(...he(l, e.issues)), n.value[u] = e.value;
				})) : (d.issues.length && n.issues.push(...he(l, d.issues)), n.value[u] = d.value);
			}
			let l;
			for (let e in i) c.has(e) || (l ??= [], l.push(e));
			l && l.length > 0 && n.issues.push({
				code: "unrecognized_keys",
				input: i,
				inst: e,
				keys: l
			});
		} else {
			n.value = {};
			for (let o of Reflect.ownKeys(i)) {
				if (o === "__proto__" || !Object.prototype.propertyIsEnumerable.call(i, o)) continue;
				let c = t.keyType._zod.run({
					value: o,
					issues: []
				}, r);
				if (c instanceof Promise) throw Error("Async schemas not supported in object keys currently");
				if (typeof o == "string" && dt.test(o) && c.issues.length) {
					let e = t.keyType._zod.run({
						value: Number(o),
						issues: []
					}, r);
					if (e instanceof Promise) throw Error("Async schemas not supported in object keys currently");
					e.issues.length === 0 && (c = e);
				}
				if (c.issues.length) {
					t.mode === "loose" ? n.value[o] = i[o] : n.issues.push({
						code: "invalid_key",
						origin: "record",
						issues: c.issues.map((e) => T(e, r, s())),
						input: o,
						path: [o],
						inst: e
					});
					continue;
				}
				let l = t.valueType._zod.run({
					value: i[o],
					issues: []
				}, r);
				l instanceof Promise ? a.push(l.then((e) => {
					e.issues.length && n.issues.push(...he(o, e.issues)), n.value[c.value] = e.value;
				})) : (l.issues.length && n.issues.push(...he(o, l.issues)), n.value[c.value] = l.value);
			}
		}
		return a.length ? Promise.all(a).then(() => n) : n;
	};
}), En = /* @__PURE__ */ r("$ZodEnum", (e, t) => {
	D.init(e, t);
	let n = c(t.entries), r = new Set(n);
	e._zod.values = r, e._zod.pattern = RegExp(`^(${n.filter((e) => ne.has(typeof e)).map((e) => typeof e == "string" ? re(e) : e.toString()).join("|")})$`), e._zod.parse = (t, i) => {
		let a = t.value;
		return r.has(a) || t.issues.push({
			code: "invalid_value",
			values: n,
			input: a,
			inst: e
		}), t;
	};
}), Dn = /* @__PURE__ */ r("$ZodLiteral", (e, t) => {
	if (D.init(e, t), t.values.length === 0) throw Error("Cannot create literal schema with no valid values");
	let n = new Set(t.values);
	e._zod.values = n, e._zod.pattern = RegExp(`^(${t.values.map((e) => typeof e == "string" ? re(e) : e ? re(e.toString()) : String(e)).join("|")})$`), e._zod.parse = (r, i) => {
		let a = r.value;
		return n.has(a) || r.issues.push({
			code: "invalid_value",
			values: t.values,
			input: a,
			inst: e
		}), r;
	};
}), On = /* @__PURE__ */ r("$ZodTransform", (e, t) => {
	D.init(e, t), e._zod.optin = "optional", e._zod.parse = (n, r) => {
		if (r.direction === "backward") throw new a(e.constructor.name);
		let o = t.transform(n.value, n);
		if (r.async) return (o instanceof Promise ? o : Promise.resolve(o)).then((e) => (n.value = e, n.fallback = !0, n));
		if (o instanceof Promise) throw new i();
		return n.value = o, n.fallback = !0, n;
	};
});
function kn(e, t) {
	return t === void 0 && (e.issues.length || e.fallback) ? {
		issues: [],
		value: void 0
	} : e;
}
var An = /* @__PURE__ */ r("$ZodOptional", (e, t) => {
	D.init(e, t), e._zod.optin = "optional", e._zod.optout = "optional", h(e._zod, "values", () => t.innerType._zod.values ? new Set([...t.innerType._zod.values, void 0]) : void 0), h(e._zod, "pattern", () => {
		let e = t.innerType._zod.pattern;
		return e ? RegExp(`^(${f(e.source)})?$`) : void 0;
	}), e._zod.parse = (e, n) => {
		if (t.innerType._zod.optin === "optional") {
			let r = e.value, i = t.innerType._zod.run(e, n);
			return i instanceof Promise ? i.then((e) => kn(e, r)) : kn(i, r);
		}
		return e.value === void 0 ? e : t.innerType._zod.run(e, n);
	};
}), jn = /* @__PURE__ */ r("$ZodExactOptional", (e, t) => {
	An.init(e, t), h(e._zod, "values", () => t.innerType._zod.values), h(e._zod, "pattern", () => t.innerType._zod.pattern), e._zod.parse = (e, n) => t.innerType._zod.run(e, n);
}), Mn = /* @__PURE__ */ r("$ZodNullable", (e, t) => {
	D.init(e, t), h(e._zod, "optin", () => t.innerType._zod.optin), h(e._zod, "optout", () => t.innerType._zod.optout), h(e._zod, "pattern", () => {
		let e = t.innerType._zod.pattern;
		return e ? RegExp(`^(${f(e.source)}|null)$`) : void 0;
	}), h(e._zod, "values", () => t.innerType._zod.values ? new Set([...t.innerType._zod.values, null]) : void 0), e._zod.parse = (e, n) => e.value === null ? e : t.innerType._zod.run(e, n);
}), Nn = /* @__PURE__ */ r("$ZodDefault", (e, t) => {
	D.init(e, t), e._zod.optin = "optional", h(e._zod, "values", () => t.innerType._zod.values), e._zod.parse = (e, n) => {
		if (n.direction === "backward") return t.innerType._zod.run(e, n);
		if (e.value === void 0) return e.value = t.defaultValue, e;
		let r = t.innerType._zod.run(e, n);
		return r instanceof Promise ? r.then((e) => Pn(e, t)) : Pn(r, t);
	};
});
function Pn(e, t) {
	return e.value === void 0 && (e.value = t.defaultValue), e;
}
var Fn = /* @__PURE__ */ r("$ZodPrefault", (e, t) => {
	D.init(e, t), e._zod.optin = "optional", h(e._zod, "values", () => t.innerType._zod.values), e._zod.parse = (e, n) => (n.direction === "backward" || e.value === void 0 && (e.value = t.defaultValue), t.innerType._zod.run(e, n));
}), In = /* @__PURE__ */ r("$ZodNonOptional", (e, t) => {
	D.init(e, t), h(e._zod, "values", () => {
		let e = t.innerType._zod.values;
		return e ? new Set([...e].filter((e) => e !== void 0)) : void 0;
	}), e._zod.parse = (n, r) => {
		let i = t.innerType._zod.run(n, r);
		return i instanceof Promise ? i.then((t) => Ln(t, e)) : Ln(i, e);
	};
});
function Ln(e, t) {
	return !e.issues.length && e.value === void 0 && e.issues.push({
		code: "invalid_type",
		expected: "nonoptional",
		input: e.value,
		inst: t
	}), e;
}
var Rn = /* @__PURE__ */ r("$ZodCatch", (e, t) => {
	D.init(e, t), e._zod.optin = "optional", h(e._zod, "optout", () => t.innerType._zod.optout), h(e._zod, "values", () => t.innerType._zod.values), e._zod.parse = (e, n) => {
		if (n.direction === "backward") return t.innerType._zod.run(e, n);
		let r = t.innerType._zod.run(e, n);
		return r instanceof Promise ? r.then((r) => (e.value = r.value, r.issues.length && (e.value = t.catchValue({
			...e,
			error: { issues: r.issues.map((e) => T(e, n, s())) },
			input: e.value
		}), e.issues = [], e.fallback = !0), e)) : (e.value = r.value, r.issues.length && (e.value = t.catchValue({
			...e,
			error: { issues: r.issues.map((e) => T(e, n, s())) },
			input: e.value
		}), e.issues = [], e.fallback = !0), e);
	};
}), zn = /* @__PURE__ */ r("$ZodPipe", (e, t) => {
	D.init(e, t), h(e._zod, "values", () => t.in._zod.values), h(e._zod, "optin", () => t.in._zod.optin), h(e._zod, "optout", () => t.out._zod.optout), h(e._zod, "propValues", () => t.in._zod.propValues), e._zod.parse = (e, n) => {
		if (n.direction === "backward") {
			let r = t.out._zod.run(e, n);
			return r instanceof Promise ? r.then((e) => Bn(e, t.in, n)) : Bn(r, t.in, n);
		}
		let r = t.in._zod.run(e, n);
		return r instanceof Promise ? r.then((e) => Bn(e, t.out, n)) : Bn(r, t.out, n);
	};
});
function Bn(e, t, n) {
	return e.issues.length ? (e.aborted = !0, e) : t._zod.run({
		value: e.value,
		issues: e.issues,
		fallback: e.fallback
	}, n);
}
var Vn = /* @__PURE__ */ r("$ZodReadonly", (e, t) => {
	D.init(e, t), h(e._zod, "propValues", () => t.innerType._zod.propValues), h(e._zod, "values", () => t.innerType._zod.values), h(e._zod, "optin", () => t.innerType?._zod?.optin), h(e._zod, "optout", () => t.innerType?._zod?.optout), e._zod.parse = (e, n) => {
		if (n.direction === "backward") return t.innerType._zod.run(e, n);
		let r = t.innerType._zod.run(e, n);
		return r instanceof Promise ? r.then(Hn) : Hn(r);
	};
});
function Hn(e) {
	return e.value = Object.freeze(e.value), e;
}
var Un = /* @__PURE__ */ r("$ZodCustom", (e, t) => {
	E.init(e, t), D.init(e, t), e._zod.parse = (e, t) => e, e._zod.check = (n) => {
		let r = n.value, i = t.fn(r);
		if (i instanceof Promise) return i.then((t) => Wn(t, n, r, e));
		Wn(i, n, r, e);
	};
});
function Wn(e, t, n, r) {
	if (!e) {
		let e = {
			code: "custom",
			input: n,
			inst: r,
			path: [...r._zod.def.path ?? []],
			continue: !r._zod.def.abort
		};
		r._zod.def.params && (e.params = r._zod.def.params), t.issues.push(ve(e));
	}
}
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/registries.js
var Gn, Kn = class {
	constructor() {
		this._map = /* @__PURE__ */ new WeakMap(), this._idmap = /* @__PURE__ */ new Map();
	}
	add(e, ...t) {
		let n = t[0];
		return this._map.set(e, n), n && typeof n == "object" && "id" in n && this._idmap.set(n.id, e), this;
	}
	clear() {
		return this._map = /* @__PURE__ */ new WeakMap(), this._idmap = /* @__PURE__ */ new Map(), this;
	}
	remove(e) {
		let t = this._map.get(e);
		return t && typeof t == "object" && "id" in t && this._idmap.delete(t.id), this._map.delete(e), this;
	}
	get(e) {
		let t = e._zod.parent;
		if (t) {
			let n = { ...this.get(t) ?? {} };
			delete n.id;
			let r = {
				...n,
				...this._map.get(e)
			};
			return Object.keys(r).length ? r : void 0;
		}
		return this._map.get(e);
	}
	has(e) {
		return this._map.has(e);
	}
};
function qn() {
	return new Kn();
}
(Gn = globalThis).__zod_globalRegistry ?? (Gn.__zod_globalRegistry = qn());
var Jn = globalThis.__zod_globalRegistry;
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/api.js
/* @__NO_SIDE_EFFECTS__ */
function Yn(e, t) {
	return new e({
		type: "string",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Xn(e, t) {
	return new e({
		type: "string",
		format: "email",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Zn(e, t) {
	return new e({
		type: "string",
		format: "guid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Qn(e, t) {
	return new e({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function $n(e, t) {
	return new e({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: !1,
		version: "v4",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function er(e, t) {
	return new e({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: !1,
		version: "v6",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function tr(e, t) {
	return new e({
		type: "string",
		format: "uuid",
		check: "string_format",
		abort: !1,
		version: "v7",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function nr(e, t) {
	return new e({
		type: "string",
		format: "url",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function rr(e, t) {
	return new e({
		type: "string",
		format: "emoji",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function ir(e, t) {
	return new e({
		type: "string",
		format: "nanoid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function ar(e, t) {
	return new e({
		type: "string",
		format: "cuid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function or(e, t) {
	return new e({
		type: "string",
		format: "cuid2",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function sr(e, t) {
	return new e({
		type: "string",
		format: "ulid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function cr(e, t) {
	return new e({
		type: "string",
		format: "xid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function lr(e, t) {
	return new e({
		type: "string",
		format: "ksuid",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function ur(e, t) {
	return new e({
		type: "string",
		format: "ipv4",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function dr(e, t) {
	return new e({
		type: "string",
		format: "ipv6",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function fr(e, t) {
	return new e({
		type: "string",
		format: "cidrv4",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function pr(e, t) {
	return new e({
		type: "string",
		format: "cidrv6",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function mr(e, t) {
	return new e({
		type: "string",
		format: "base64",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function hr(e, t) {
	return new e({
		type: "string",
		format: "base64url",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function gr(e, t) {
	return new e({
		type: "string",
		format: "e164",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function _r(e, t) {
	return new e({
		type: "string",
		format: "jwt",
		check: "string_format",
		abort: !1,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function vr(e, t) {
	return new e({
		type: "string",
		format: "datetime",
		check: "string_format",
		offset: !1,
		local: !1,
		precision: null,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function yr(e, t) {
	return new e({
		type: "string",
		format: "date",
		check: "string_format",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function br(e, t) {
	return new e({
		type: "string",
		format: "time",
		check: "string_format",
		precision: null,
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function xr(e, t) {
	return new e({
		type: "string",
		format: "duration",
		check: "string_format",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Sr(e, t) {
	return new e({
		type: "number",
		checks: [],
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Cr(e, t) {
	return new e({
		type: "number",
		check: "number_format",
		abort: !1,
		format: "safeint",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function wr(e, t) {
	return new e({
		type: "boolean",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Tr(e) {
	return new e({ type: "unknown" });
}
/* @__NO_SIDE_EFFECTS__ */
function Er(e, t) {
	return new e({
		type: "never",
		...w(t)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Dr(e, t) {
	return new gt({
		check: "less_than",
		...w(t),
		value: e,
		inclusive: !1
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Or(e, t) {
	return new gt({
		check: "less_than",
		...w(t),
		value: e,
		inclusive: !0
	});
}
/* @__NO_SIDE_EFFECTS__ */
function kr(e, t) {
	return new _t({
		check: "greater_than",
		...w(t),
		value: e,
		inclusive: !1
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Ar(e, t) {
	return new _t({
		check: "greater_than",
		...w(t),
		value: e,
		inclusive: !0
	});
}
/* @__NO_SIDE_EFFECTS__ */
function jr(e, t) {
	return new vt({
		check: "multiple_of",
		...w(t),
		value: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Mr(e, t) {
	return new bt({
		check: "max_length",
		...w(t),
		maximum: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Nr(e, t) {
	return new xt({
		check: "min_length",
		...w(t),
		minimum: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Pr(e, t) {
	return new St({
		check: "length_equals",
		...w(t),
		length: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Fr(e, t) {
	return new wt({
		check: "string_format",
		format: "regex",
		...w(t),
		pattern: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Ir(e) {
	return new Tt({
		check: "string_format",
		format: "lowercase",
		...w(e)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Lr(e) {
	return new Et({
		check: "string_format",
		format: "uppercase",
		...w(e)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Rr(e, t) {
	return new Dt({
		check: "string_format",
		format: "includes",
		...w(t),
		includes: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function zr(e, t) {
	return new Ot({
		check: "string_format",
		format: "starts_with",
		...w(t),
		prefix: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Br(e, t) {
	return new kt({
		check: "string_format",
		format: "ends_with",
		...w(t),
		suffix: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Vr(e) {
	return new At({
		check: "overwrite",
		tx: e
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Hr(e) {
	return /* @__PURE__ */ Vr((t) => t.normalize(e));
}
/* @__NO_SIDE_EFFECTS__ */
function Ur() {
	return /* @__PURE__ */ Vr((e) => e.trim());
}
/* @__NO_SIDE_EFFECTS__ */
function Wr() {
	return /* @__PURE__ */ Vr((e) => e.toLowerCase());
}
/* @__NO_SIDE_EFFECTS__ */
function Gr() {
	return /* @__PURE__ */ Vr((e) => e.toUpperCase());
}
/* @__NO_SIDE_EFFECTS__ */
function Kr() {
	return /* @__PURE__ */ Vr((e) => y(e));
}
/* @__NO_SIDE_EFFECTS__ */
function qr(e, t, n) {
	return new e({
		type: "array",
		element: t,
		...w(n)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Jr(e, t, n) {
	return new e({
		type: "custom",
		check: "custom",
		fn: t,
		...w(n)
	});
}
/* @__NO_SIDE_EFFECTS__ */
function Yr(e, t) {
	let n = /* @__PURE__ */ Xr((t) => (t.addIssue = (e) => {
		if (typeof e == "string") t.issues.push(ve(e, t.value, n._zod.def));
		else {
			let r = e;
			r.fatal && (r.continue = !1), r.code ??= "custom", r.input ??= t.value, r.inst ??= n, r.continue ??= !n._zod.def.abort, t.issues.push(ve(r));
		}
	}, e(t.value, t)), t);
	return n;
}
/* @__NO_SIDE_EFFECTS__ */
function Xr(e, t) {
	let n = new E({
		check: "custom",
		...w(t)
	});
	return n._zod.check = e, n;
}
//#endregion
//#region node_modules/.pnpm/zod@4.4.3/node_modules/zod/v4/core/to-json-schema.js
function Zr(e) {
	let t = e?.target ?? "draft-2020-12";
	return t === "draft-4" && (t = "draft-04"), t === "draft-7" && (t = "draft-07"), {
		processors: e.processors ?? {},
		metadataRegistry: e?.metadata ?? Jn,
		target: t,
		unrepresentable: e?.unrepresentable ?? "throw",
		override: e?.override ?? (() => {}),
		io: e?.io ?? "output",
		counter: 0,
		seen: /* @__PURE__ */ new Map(),
		cycles: e?.cycles ?? "ref",
		reused: e?.reused ?? "inline",
		external: e?.external ?? void 0
	};
}
function k(e, t, n = {
	path: [],
	schemaPath: []
}) {
	var r;
	let i = e._zod.def, a = t.seen.get(e);
	if (a) return a.count++, n.schemaPath.includes(e) && (a.cycle = n.path), a.schema;
	let o = {
		schema: {},
		count: 1,
		cycle: void 0,
		path: n.path
	};
	t.seen.set(e, o);
	let s = e._zod.toJSONSchema?.();
	if (s) o.schema = s;
	else {
		let r = {
			...n,
			schemaPath: [...n.schemaPath, e],
			path: n.path
		};
		if (e._zod.processJSONSchema) e._zod.processJSONSchema(t, o.schema, r);
		else {
			let n = o.schema, a = t.processors[i.type];
			if (!a) throw Error(`[toJSONSchema]: Non-representable type encountered: ${i.type}`);
			a(e, t, n, r);
		}
		let a = e._zod.parent;
		a && (o.ref ||= a, k(a, t, r), t.seen.get(a).isParent = !0);
	}
	let c = t.metadataRegistry.get(e);
	return c && Object.assign(o.schema, c), t.io === "input" && A(e) && (delete o.schema.examples, delete o.schema.default), t.io === "input" && "_prefault" in o.schema && ((r = o.schema).default ?? (r.default = o.schema._prefault)), delete o.schema._prefault, t.seen.get(e).schema;
}
function Qr(e, t) {
	let n = e.seen.get(t);
	if (!n) throw Error("Unprocessed schema. This is a bug in Zod.");
	let r = /* @__PURE__ */ new Map();
	for (let t of e.seen.entries()) {
		let n = e.metadataRegistry.get(t[0])?.id;
		if (n) {
			let e = r.get(n);
			if (e && e !== t[0]) throw Error(`Duplicate schema id "${n}" detected during JSON Schema conversion. Two different schemas cannot share the same id when converted together.`);
			r.set(n, t[0]);
		}
	}
	let i = (t) => {
		let r = e.target === "draft-2020-12" ? "$defs" : "definitions";
		if (e.external) {
			let n = e.external.registry.get(t[0])?.id, i = e.external.uri ?? ((e) => e);
			if (n) return { ref: i(n) };
			let a = t[1].defId ?? t[1].schema.id ?? `schema${e.counter++}`;
			return t[1].defId = a, {
				defId: a,
				ref: `${i("__shared")}#/${r}/${a}`
			};
		}
		if (t[1] === n) return { ref: "#" };
		let i = `#/${r}/`, a = t[1].schema.id ?? `__schema${e.counter++}`;
		return {
			defId: a,
			ref: i + a
		};
	}, a = (e) => {
		if (e[1].schema.$ref) return;
		let t = e[1], { ref: n, defId: r } = i(e);
		t.def = { ...t.schema }, r && (t.defId = r);
		let a = t.schema;
		for (let e in a) delete a[e];
		a.$ref = n;
	};
	if (e.cycles === "throw") for (let t of e.seen.entries()) {
		let e = t[1];
		if (e.cycle) throw Error(`Cycle detected: #/${e.cycle?.join("/")}/<root>

Set the \`cycles\` parameter to \`"ref"\` to resolve cyclical schemas with defs.`);
	}
	for (let n of e.seen.entries()) {
		let r = n[1];
		if (t === n[0]) {
			a(n);
			continue;
		}
		if (e.external) {
			let r = e.external.registry.get(n[0])?.id;
			if (t !== n[0] && r) {
				a(n);
				continue;
			}
		}
		if (e.metadataRegistry.get(n[0])?.id) {
			a(n);
			continue;
		}
		if (r.cycle) {
			a(n);
			continue;
		}
		if (r.count > 1 && e.reused === "ref") {
			a(n);
			continue;
		}
	}
}
function $r(e, t) {
	let n = e.seen.get(t);
	if (!n) throw Error("Unprocessed schema. This is a bug in Zod.");
	let r = (t) => {
		let n = e.seen.get(t);
		if (n.ref === null) return;
		let i = n.def ?? n.schema, a = { ...i }, o = n.ref;
		if (n.ref = null, o) {
			r(o);
			let n = e.seen.get(o), s = n.schema;
			if (s.$ref && (e.target === "draft-07" || e.target === "draft-04" || e.target === "openapi-3.0") ? (i.allOf = i.allOf ?? [], i.allOf.push(s)) : Object.assign(i, s), Object.assign(i, a), t._zod.parent === o) for (let e in i) e === "$ref" || e === "allOf" || e in a || delete i[e];
			if (s.$ref && n.def) for (let e in i) e === "$ref" || e === "allOf" || e in n.def && JSON.stringify(i[e]) === JSON.stringify(n.def[e]) && delete i[e];
		}
		let s = t._zod.parent;
		if (s && s !== o) {
			r(s);
			let t = e.seen.get(s);
			if (t?.schema.$ref && (i.$ref = t.schema.$ref, t.def)) for (let e in i) e === "$ref" || e === "allOf" || e in t.def && JSON.stringify(i[e]) === JSON.stringify(t.def[e]) && delete i[e];
		}
		e.override({
			zodSchema: t,
			jsonSchema: i,
			path: n.path ?? []
		});
	};
	for (let t of [...e.seen.entries()].reverse()) r(t[0]);
	let i = {};
	if (e.target === "draft-2020-12" ? i.$schema = "https://json-schema.org/draft/2020-12/schema" : e.target === "draft-07" ? i.$schema = "http://json-schema.org/draft-07/schema#" : e.target === "draft-04" ? i.$schema = "http://json-schema.org/draft-04/schema#" : e.target, e.external?.uri) {
		let n = e.external.registry.get(t)?.id;
		if (!n) throw Error("Schema is missing an `id` property");
		i.$id = e.external.uri(n);
	}
	Object.assign(i, n.def ?? n.schema);
	let a = e.metadataRegistry.get(t)?.id;
	a !== void 0 && i.id === a && delete i.id;
	let o = e.external?.defs ?? {};
	for (let t of e.seen.entries()) {
		let e = t[1];
		e.def && e.defId && (e.def.id === e.defId && delete e.def.id, o[e.defId] = e.def);
	}
	e.external || Object.keys(o).length > 0 && (e.target === "draft-2020-12" ? i.$defs = o : i.definitions = o);
	try {
		let n = JSON.parse(JSON.stringify(i));
		return Object.defineProperty(n, "~standard", {
			value: {
				...t["~standard"],
				jsonSchema: {
					input: ti(t, "input", e.processors),
					output: ti(t, "output", e.processors)
				}
			},
			enumerable: !1,
			writable: !1
		}), n;
	} catch {
		throw Error("Error converting schema to JSON.");
	}
}
function A(e, t) {
	let n = t ?? { seen: /* @__PURE__ */ new Set() };
	if (n.seen.has(e)) return !1;
	n.seen.add(e);
	let r = e._zod.def;
	if (r.type === "transform") return !0;
	if (r.type === "array") return A(r.element, n);
	if (r.type === "set") return A(r.valueType, n);
	if (r.type === "lazy") return A(r.getter(), n);
	if (r.type === "promise" || r.type === "optional" || r.type === "nonoptional" || r.type === "nullable" || r.type === "readonly" || r.type === "default" || r.type === "prefault") return A(r.innerType, n);
	if (r.type === "intersection") return A(r.left, n) || A(r.right, n);
	if (r.type === "record" || r.type === "map") return A(r.keyType, n) || A(r.valueType, n);
	if (r.type === "pipe") return e._zod.traits.has("$ZodCodec") ? !0 : A(r.in, n) || A(r.out, n);
	if (r.type === "object") {
		for (let e in r.shape) if (A(r.shape[e], n)) return !0;
		return !1;
	}
	if (r.type === "union") {
		for (let e of r.options) if (A(e, n)) return !0;
		return !1;
	}
	if (r.type === "tuple") {
		for (let e of r.items) if (A(e, n)) return !0;
		return !!(r.rest && A(r.rest, n));
	}
	return !1;
}
var ei = (e, t = {}) => (n) => {
	let r = Zr({
		...n,
		processors: t
	});
	return k(e, r), Qr(r, e), $r(r, e);
}, ti = (e, t, n = {}) => (r) => {
	let { libraryOptions: i, target: a } = r ?? {}, o = Zr({
		...i ?? {},
		target: a,
		io: t,
		processors: n
	});
	return k(e, o), Qr(o, e), $r(o, e);
}, ni = {
	guid: "uuid",
	url: "uri",
	datetime: "date-time",
	json_string: "json-string",
	regex: ""
}, ri = (e, t, n, r) => {
	let i = n;
	i.type = "string";
	let { minimum: a, maximum: o, format: s, patterns: c, contentEncoding: l } = e._zod.bag;
	if (typeof a == "number" && (i.minLength = a), typeof o == "number" && (i.maxLength = o), s && (i.format = ni[s] ?? s, i.format === "" && delete i.format, s === "time" && delete i.format), l && (i.contentEncoding = l), c && c.size > 0) {
		let e = [...c];
		e.length === 1 ? i.pattern = e[0].source : e.length > 1 && (i.allOf = [...e.map((e) => ({
			...t.target === "draft-07" || t.target === "draft-04" || t.target === "openapi-3.0" ? { type: "string" } : {},
			pattern: e.source
		}))]);
	}
}, ii = (e, t, n, r) => {
	let i = n, { minimum: a, maximum: o, format: s, multipleOf: c, exclusiveMaximum: l, exclusiveMinimum: u } = e._zod.bag;
	typeof s == "string" && s.includes("int") ? i.type = "integer" : i.type = "number";
	let d = typeof u == "number" && u >= (a ?? -Infinity), f = typeof l == "number" && l <= (o ?? Infinity), p = t.target === "draft-04" || t.target === "openapi-3.0";
	d ? p ? (i.minimum = u, i.exclusiveMinimum = !0) : i.exclusiveMinimum = u : typeof a == "number" && (i.minimum = a), f ? p ? (i.maximum = l, i.exclusiveMaximum = !0) : i.exclusiveMaximum = l : typeof o == "number" && (i.maximum = o), typeof c == "number" && (i.multipleOf = c);
}, ai = (e, t, n, r) => {
	n.type = "boolean";
}, oi = (e, t, n, r) => {
	n.not = {};
}, si = (e, t, n, r) => {
	let i = e._zod.def, a = c(i.entries);
	a.every((e) => typeof e == "number") && (n.type = "number"), a.every((e) => typeof e == "string") && (n.type = "string"), n.enum = a;
}, ci = (e, t, n, r) => {
	let i = e._zod.def, a = [];
	for (let e of i.values) if (e === void 0) {
		if (t.unrepresentable === "throw") throw Error("Literal `undefined` cannot be represented in JSON Schema");
	} else if (typeof e == "bigint") {
		if (t.unrepresentable === "throw") throw Error("BigInt literals cannot be represented in JSON Schema");
		a.push(Number(e));
	} else a.push(e);
	if (a.length !== 0) if (a.length === 1) {
		let e = a[0];
		n.type = e === null ? "null" : typeof e, t.target === "draft-04" || t.target === "openapi-3.0" ? n.enum = [e] : n.const = e;
	} else a.every((e) => typeof e == "number") && (n.type = "number"), a.every((e) => typeof e == "string") && (n.type = "string"), a.every((e) => typeof e == "boolean") && (n.type = "boolean"), a.every((e) => e === null) && (n.type = "null"), n.enum = a;
}, li = (e, t, n, r) => {
	if (t.unrepresentable === "throw") throw Error("Custom types cannot be represented in JSON Schema");
}, ui = (e, t, n, r) => {
	if (t.unrepresentable === "throw") throw Error("Transforms cannot be represented in JSON Schema");
}, di = (e, t, n, r) => {
	let i = n, a = e._zod.def, { minimum: o, maximum: s } = e._zod.bag;
	typeof o == "number" && (i.minItems = o), typeof s == "number" && (i.maxItems = s), i.type = "array", i.items = k(a.element, t, {
		...r,
		path: [...r.path, "items"]
	});
}, fi = (e, t, n, r) => {
	let i = n, a = e._zod.def;
	i.type = "object", i.properties = {};
	let o = a.shape;
	for (let e in o) i.properties[e] = k(o[e], t, {
		...r,
		path: [
			...r.path,
			"properties",
			e
		]
	});
	let s = new Set(Object.keys(o)), c = new Set([...s].filter((e) => {
		let n = a.shape[e]._zod;
		return t.io === "input" ? n.optin === void 0 : n.optout === void 0;
	}));
	c.size > 0 && (i.required = Array.from(c)), a.catchall?._zod.def.type === "never" ? i.additionalProperties = !1 : a.catchall ? a.catchall && (i.additionalProperties = k(a.catchall, t, {
		...r,
		path: [...r.path, "additionalProperties"]
	})) : t.io === "output" && (i.additionalProperties = !1);
}, pi = (e, t, n, r) => {
	let i = e._zod.def, a = i.inclusive === !1, o = i.options.map((e, n) => k(e, t, {
		...r,
		path: [
			...r.path,
			a ? "oneOf" : "anyOf",
			n
		]
	}));
	a ? n.oneOf = o : n.anyOf = o;
}, mi = (e, t, n, r) => {
	let i = e._zod.def, a = k(i.left, t, {
		...r,
		path: [
			...r.path,
			"allOf",
			0
		]
	}), o = k(i.right, t, {
		...r,
		path: [
			...r.path,
			"allOf",
			1
		]
	}), s = (e) => "allOf" in e && Object.keys(e).length === 1;
	n.allOf = [...s(a) ? a.allOf : [a], ...s(o) ? o.allOf : [o]];
}, hi = (e, t, n, r) => {
	let i = n, a = e._zod.def;
	i.type = "object";
	let o = a.keyType, s = o._zod.bag?.patterns;
	if (a.mode === "loose" && s && s.size > 0) {
		let e = k(a.valueType, t, {
			...r,
			path: [
				...r.path,
				"patternProperties",
				"*"
			]
		});
		i.patternProperties = {};
		for (let t of s) i.patternProperties[t.source] = e;
	} else (t.target === "draft-07" || t.target === "draft-2020-12") && (i.propertyNames = k(a.keyType, t, {
		...r,
		path: [...r.path, "propertyNames"]
	})), i.additionalProperties = k(a.valueType, t, {
		...r,
		path: [...r.path, "additionalProperties"]
	});
	let c = o._zod.values;
	if (c) {
		let e = [...c].filter((e) => typeof e == "string" || typeof e == "number");
		e.length > 0 && (i.required = e);
	}
}, gi = (e, t, n, r) => {
	let i = e._zod.def, a = k(i.innerType, t, r), o = t.seen.get(e);
	t.target === "openapi-3.0" ? (o.ref = i.innerType, n.nullable = !0) : n.anyOf = [a, { type: "null" }];
}, _i = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType;
}, vi = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType, n.default = JSON.parse(JSON.stringify(i.defaultValue));
}, yi = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType, t.io === "input" && (n._prefault = JSON.parse(JSON.stringify(i.defaultValue)));
}, bi = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType;
	let o;
	try {
		o = i.catchValue(void 0);
	} catch {
		throw Error("Dynamic catch values are not supported in JSON Schema");
	}
	n.default = o;
}, xi = (e, t, n, r) => {
	let i = e._zod.def, a = i.in._zod.traits.has("$ZodTransform"), o = t.io === "input" ? a ? i.out : i.in : i.out;
	k(o, t, r);
	let s = t.seen.get(e);
	s.ref = o;
}, Si = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType, n.readOnly = !0;
}, Ci = (e, t, n, r) => {
	let i = e._zod.def;
	k(i.innerType, t, r);
	let a = t.seen.get(e);
	a.ref = i.innerType;
}, wi = /* @__PURE__ */ r("ZodISODateTime", (e, t) => {
	Gt.init(e, t), P.init(e, t);
});
function Ti(e) {
	return /* @__PURE__ */ vr(wi, e);
}
var Ei = /* @__PURE__ */ r("ZodISODate", (e, t) => {
	Kt.init(e, t), P.init(e, t);
});
function Di(e) {
	return /* @__PURE__ */ yr(Ei, e);
}
var Oi = /* @__PURE__ */ r("ZodISOTime", (e, t) => {
	qt.init(e, t), P.init(e, t);
});
function ki(e) {
	return /* @__PURE__ */ br(Oi, e);
}
var Ai = /* @__PURE__ */ r("ZodISODuration", (e, t) => {
	Jt.init(e, t), P.init(e, t);
});
function ji(e) {
	return /* @__PURE__ */ xr(Ai, e);
}
var j = /* @__PURE__ */ r("ZodError", (e, t) => {
	be.init(e, t), e.name = "ZodError", Object.defineProperties(e, {
		format: { value: (t) => Ce(e, t) },
		flatten: { value: (t) => Se(e, t) },
		addIssue: { value: (t) => {
			e.issues.push(t), e.message = JSON.stringify(e.issues, l, 2);
		} },
		addIssues: { value: (t) => {
			e.issues.push(...t), e.message = JSON.stringify(e.issues, l, 2);
		} },
		isEmpty: { get() {
			return e.issues.length === 0;
		} }
	});
}, { Parent: Error }), Mi = /* @__PURE__ */ we(j), Ni = /* @__PURE__ */ Te(j), Pi = /* @__PURE__ */ Ee(j), Fi = /* @__PURE__ */ Oe(j), Ii = /* @__PURE__ */ Ae(j), Li = /* @__PURE__ */ je(j), Ri = /* @__PURE__ */ Me(j), zi = /* @__PURE__ */ Ne(j), Bi = /* @__PURE__ */ Pe(j), Vi = /* @__PURE__ */ Fe(j), Hi = /* @__PURE__ */ Ie(j), Ui = /* @__PURE__ */ Le(j), Wi = /* @__PURE__ */ new WeakMap();
function Gi(e, t, n) {
	let r = Object.getPrototypeOf(e), i = Wi.get(r);
	if (i || (i = /* @__PURE__ */ new Set(), Wi.set(r, i)), !i.has(t)) {
		i.add(t);
		for (let e in n) {
			let t = n[e];
			Object.defineProperty(r, e, {
				configurable: !0,
				enumerable: !1,
				get() {
					let n = t.bind(this);
					return Object.defineProperty(this, e, {
						configurable: !0,
						writable: !0,
						enumerable: !0,
						value: n
					}), n;
				},
				set(t) {
					Object.defineProperty(this, e, {
						configurable: !0,
						writable: !0,
						enumerable: !0,
						value: t
					});
				}
			});
		}
	}
}
var M = /* @__PURE__ */ r("ZodType", (e, t) => (D.init(e, t), Object.assign(e["~standard"], { jsonSchema: {
	input: ti(e, "input"),
	output: ti(e, "output")
} }), e.toJSONSchema = ei(e, {}), e.def = t, e.type = t.type, Object.defineProperty(e, "_def", { value: t }), e.parse = (t, n) => Mi(e, t, n, { callee: e.parse }), e.safeParse = (t, n) => Pi(e, t, n), e.parseAsync = async (t, n) => Ni(e, t, n, { callee: e.parseAsync }), e.safeParseAsync = async (t, n) => Fi(e, t, n), e.spa = e.safeParseAsync, e.encode = (t, n) => Ii(e, t, n), e.decode = (t, n) => Li(e, t, n), e.encodeAsync = async (t, n) => Ri(e, t, n), e.decodeAsync = async (t, n) => zi(e, t, n), e.safeEncode = (t, n) => Bi(e, t, n), e.safeDecode = (t, n) => Vi(e, t, n), e.safeEncodeAsync = async (t, n) => Hi(e, t, n), e.safeDecodeAsync = async (t, n) => Ui(e, t, n), Gi(e, "ZodType", {
	check(...e) {
		let t = this.def;
		return this.clone(_(t, { checks: [...t.checks ?? [], ...e.map((e) => typeof e == "function" ? { _zod: {
			check: e,
			def: { check: "custom" },
			onattach: []
		} } : e)] }), { parent: !0 });
	},
	with(...e) {
		return this.check(...e);
	},
	clone(e, t) {
		return C(this, e, t);
	},
	brand() {
		return this;
	},
	register(e, t) {
		return e.add(this, t), this;
	},
	refine(e, t) {
		return this.check(io(e, t));
	},
	superRefine(e, t) {
		return this.check(ao(e, t));
	},
	overwrite(e) {
		return this.check(/* @__PURE__ */ Vr(e));
	},
	optional() {
		return Ba(this);
	},
	exactOptional() {
		return Ha(this);
	},
	nullable() {
		return Wa(this);
	},
	nullish() {
		return Ba(Wa(this));
	},
	nonoptional(e) {
		return Xa(this, e);
	},
	array() {
		return Ca(this);
	},
	or(e) {
		return Ea([this, e]);
	},
	and(e) {
		return Aa(this, e);
	},
	transform(e) {
		return eo(this, Ra(e));
	},
	default(e) {
		return Ka(this, e);
	},
	prefault(e) {
		return Ja(this, e);
	},
	catch(e) {
		return Qa(this, e);
	},
	pipe(e) {
		return eo(this, e);
	},
	readonly() {
		return no(this);
	},
	describe(e) {
		let t = this.clone();
		return Jn.add(t, { description: e }), t;
	},
	meta(...e) {
		if (e.length === 0) return Jn.get(this);
		let t = this.clone();
		return Jn.add(t, e[0]), t;
	},
	isOptional() {
		return this.safeParse(void 0).success;
	},
	isNullable() {
		return this.safeParse(null).success;
	},
	apply(e) {
		return e(this);
	}
}), Object.defineProperty(e, "description", {
	get() {
		return Jn.get(e)?.description;
	},
	configurable: !0
}), e)), Ki = /* @__PURE__ */ r("_ZodString", (e, t) => {
	Nt.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => ri(e, t, n, r);
	let n = e._zod.bag;
	e.format = n.format ?? null, e.minLength = n.minimum ?? null, e.maxLength = n.maximum ?? null, Gi(e, "_ZodString", {
		regex(...e) {
			return this.check(/* @__PURE__ */ Fr(...e));
		},
		includes(...e) {
			return this.check(/* @__PURE__ */ Rr(...e));
		},
		startsWith(...e) {
			return this.check(/* @__PURE__ */ zr(...e));
		},
		endsWith(...e) {
			return this.check(/* @__PURE__ */ Br(...e));
		},
		min(...e) {
			return this.check(/* @__PURE__ */ Nr(...e));
		},
		max(...e) {
			return this.check(/* @__PURE__ */ Mr(...e));
		},
		length(...e) {
			return this.check(/* @__PURE__ */ Pr(...e));
		},
		nonempty(...e) {
			return this.check(/* @__PURE__ */ Nr(1, ...e));
		},
		lowercase(e) {
			return this.check(/* @__PURE__ */ Ir(e));
		},
		uppercase(e) {
			return this.check(/* @__PURE__ */ Lr(e));
		},
		trim() {
			return this.check(/* @__PURE__ */ Ur());
		},
		normalize(...e) {
			return this.check(/* @__PURE__ */ Hr(...e));
		},
		toLowerCase() {
			return this.check(/* @__PURE__ */ Wr());
		},
		toUpperCase() {
			return this.check(/* @__PURE__ */ Gr());
		},
		slugify() {
			return this.check(/* @__PURE__ */ Kr());
		}
	});
}), qi = /* @__PURE__ */ r("ZodString", (e, t) => {
	Nt.init(e, t), Ki.init(e, t), e.email = (t) => e.check(/* @__PURE__ */ Xn(Ji, t)), e.url = (t) => e.check(/* @__PURE__ */ nr(Zi, t)), e.jwt = (t) => e.check(/* @__PURE__ */ _r(fa, t)), e.emoji = (t) => e.check(/* @__PURE__ */ rr(Qi, t)), e.guid = (t) => e.check(/* @__PURE__ */ Zn(Yi, t)), e.uuid = (t) => e.check(/* @__PURE__ */ Qn(Xi, t)), e.uuidv4 = (t) => e.check(/* @__PURE__ */ $n(Xi, t)), e.uuidv6 = (t) => e.check(/* @__PURE__ */ er(Xi, t)), e.uuidv7 = (t) => e.check(/* @__PURE__ */ tr(Xi, t)), e.nanoid = (t) => e.check(/* @__PURE__ */ ir($i, t)), e.guid = (t) => e.check(/* @__PURE__ */ Zn(Yi, t)), e.cuid = (t) => e.check(/* @__PURE__ */ ar(ea, t)), e.cuid2 = (t) => e.check(/* @__PURE__ */ or(ta, t)), e.ulid = (t) => e.check(/* @__PURE__ */ sr(na, t)), e.base64 = (t) => e.check(/* @__PURE__ */ mr(la, t)), e.base64url = (t) => e.check(/* @__PURE__ */ hr(ua, t)), e.xid = (t) => e.check(/* @__PURE__ */ cr(ra, t)), e.ksuid = (t) => e.check(/* @__PURE__ */ lr(ia, t)), e.ipv4 = (t) => e.check(/* @__PURE__ */ ur(aa, t)), e.ipv6 = (t) => e.check(/* @__PURE__ */ dr(oa, t)), e.cidrv4 = (t) => e.check(/* @__PURE__ */ fr(sa, t)), e.cidrv6 = (t) => e.check(/* @__PURE__ */ pr(ca, t)), e.e164 = (t) => e.check(/* @__PURE__ */ gr(da, t)), e.datetime = (t) => e.check(Ti(t)), e.date = (t) => e.check(Di(t)), e.time = (t) => e.check(ki(t)), e.duration = (t) => e.check(ji(t));
});
function N(e) {
	return /* @__PURE__ */ Yn(qi, e);
}
var P = /* @__PURE__ */ r("ZodStringFormat", (e, t) => {
	O.init(e, t), Ki.init(e, t);
}), Ji = /* @__PURE__ */ r("ZodEmail", (e, t) => {
	It.init(e, t), P.init(e, t);
}), Yi = /* @__PURE__ */ r("ZodGUID", (e, t) => {
	Pt.init(e, t), P.init(e, t);
}), Xi = /* @__PURE__ */ r("ZodUUID", (e, t) => {
	Ft.init(e, t), P.init(e, t);
}), Zi = /* @__PURE__ */ r("ZodURL", (e, t) => {
	Lt.init(e, t), P.init(e, t);
}), Qi = /* @__PURE__ */ r("ZodEmoji", (e, t) => {
	Rt.init(e, t), P.init(e, t);
}), $i = /* @__PURE__ */ r("ZodNanoID", (e, t) => {
	zt.init(e, t), P.init(e, t);
}), ea = /* @__PURE__ */ r("ZodCUID", (e, t) => {
	Bt.init(e, t), P.init(e, t);
}), ta = /* @__PURE__ */ r("ZodCUID2", (e, t) => {
	Vt.init(e, t), P.init(e, t);
}), na = /* @__PURE__ */ r("ZodULID", (e, t) => {
	Ht.init(e, t), P.init(e, t);
}), ra = /* @__PURE__ */ r("ZodXID", (e, t) => {
	Ut.init(e, t), P.init(e, t);
}), ia = /* @__PURE__ */ r("ZodKSUID", (e, t) => {
	Wt.init(e, t), P.init(e, t);
}), aa = /* @__PURE__ */ r("ZodIPv4", (e, t) => {
	Yt.init(e, t), P.init(e, t);
}), oa = /* @__PURE__ */ r("ZodIPv6", (e, t) => {
	Xt.init(e, t), P.init(e, t);
}), sa = /* @__PURE__ */ r("ZodCIDRv4", (e, t) => {
	Zt.init(e, t), P.init(e, t);
}), ca = /* @__PURE__ */ r("ZodCIDRv6", (e, t) => {
	Qt.init(e, t), P.init(e, t);
}), la = /* @__PURE__ */ r("ZodBase64", (e, t) => {
	en.init(e, t), P.init(e, t);
}), ua = /* @__PURE__ */ r("ZodBase64URL", (e, t) => {
	nn.init(e, t), P.init(e, t);
}), da = /* @__PURE__ */ r("ZodE164", (e, t) => {
	rn.init(e, t), P.init(e, t);
}), fa = /* @__PURE__ */ r("ZodJWT", (e, t) => {
	on.init(e, t), P.init(e, t);
}), pa = /* @__PURE__ */ r("ZodNumber", (e, t) => {
	sn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => ii(e, t, n, r), Gi(e, "ZodNumber", {
		gt(e, t) {
			return this.check(/* @__PURE__ */ kr(e, t));
		},
		gte(e, t) {
			return this.check(/* @__PURE__ */ Ar(e, t));
		},
		min(e, t) {
			return this.check(/* @__PURE__ */ Ar(e, t));
		},
		lt(e, t) {
			return this.check(/* @__PURE__ */ Dr(e, t));
		},
		lte(e, t) {
			return this.check(/* @__PURE__ */ Or(e, t));
		},
		max(e, t) {
			return this.check(/* @__PURE__ */ Or(e, t));
		},
		int(e) {
			return this.check(ga(e));
		},
		safe(e) {
			return this.check(ga(e));
		},
		positive(e) {
			return this.check(/* @__PURE__ */ kr(0, e));
		},
		nonnegative(e) {
			return this.check(/* @__PURE__ */ Ar(0, e));
		},
		negative(e) {
			return this.check(/* @__PURE__ */ Dr(0, e));
		},
		nonpositive(e) {
			return this.check(/* @__PURE__ */ Or(0, e));
		},
		multipleOf(e, t) {
			return this.check(/* @__PURE__ */ jr(e, t));
		},
		step(e, t) {
			return this.check(/* @__PURE__ */ jr(e, t));
		},
		finite() {
			return this;
		}
	});
	let n = e._zod.bag;
	e.minValue = Math.max(n.minimum ?? -Infinity, n.exclusiveMinimum ?? -Infinity) ?? null, e.maxValue = Math.min(n.maximum ?? Infinity, n.exclusiveMaximum ?? Infinity) ?? null, e.isInt = (n.format ?? "").includes("int") || Number.isSafeInteger(n.multipleOf ?? .5), e.isFinite = !0, e.format = n.format ?? null;
});
function ma(e) {
	return /* @__PURE__ */ Sr(pa, e);
}
var ha = /* @__PURE__ */ r("ZodNumberFormat", (e, t) => {
	cn.init(e, t), pa.init(e, t);
});
function ga(e) {
	return /* @__PURE__ */ Cr(ha, e);
}
var _a = /* @__PURE__ */ r("ZodBoolean", (e, t) => {
	ln.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => ai(e, t, n, r);
});
function va(e) {
	return /* @__PURE__ */ wr(_a, e);
}
var ya = /* @__PURE__ */ r("ZodUnknown", (e, t) => {
	un.init(e, t), M.init(e, t), e._zod.processJSONSchema = (e, t, n) => void 0;
});
function F() {
	return /* @__PURE__ */ Tr(ya);
}
var ba = /* @__PURE__ */ r("ZodNever", (e, t) => {
	dn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => oi(e, t, n, r);
});
function xa(e) {
	return /* @__PURE__ */ Er(ba, e);
}
var Sa = /* @__PURE__ */ r("ZodArray", (e, t) => {
	pn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => di(e, t, n, r), e.element = t.element, Gi(e, "ZodArray", {
		min(e, t) {
			return this.check(/* @__PURE__ */ Nr(e, t));
		},
		nonempty(e) {
			return this.check(/* @__PURE__ */ Nr(1, e));
		},
		max(e, t) {
			return this.check(/* @__PURE__ */ Mr(e, t));
		},
		length(e, t) {
			return this.check(/* @__PURE__ */ Pr(e, t));
		},
		unwrap() {
			return this.element;
		}
	});
});
function Ca(e, t) {
	return /* @__PURE__ */ qr(Sa, e, t);
}
var wa = /* @__PURE__ */ r("ZodObject", (e, t) => {
	vn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => fi(e, t, n, r), h(e, "shape", () => t.shape), Gi(e, "ZodObject", {
		keyof() {
			return Pa(Object.keys(this._zod.def.shape));
		},
		catchall(e) {
			return this.clone({
				...this._zod.def,
				catchall: e
			});
		},
		passthrough() {
			return this.clone({
				...this._zod.def,
				catchall: F()
			});
		},
		loose() {
			return this.clone({
				...this._zod.def,
				catchall: F()
			});
		},
		strict() {
			return this.clone({
				...this._zod.def,
				catchall: xa()
			});
		},
		strip() {
			return this.clone({
				...this._zod.def,
				catchall: void 0
			});
		},
		extend(e) {
			return ce(this, e);
		},
		safeExtend(e) {
			return le(this, e);
		},
		merge(e) {
			return ue(this, e);
		},
		pick(e) {
			return oe(this, e);
		},
		omit(e) {
			return se(this, e);
		},
		partial(...e) {
			return de(za, this, e[0]);
		},
		required(...e) {
			return fe(Ya, this, e[0]);
		}
	});
});
function I(e, t) {
	return new wa({
		type: "object",
		shape: e ?? {},
		...w(t)
	});
}
var Ta = /* @__PURE__ */ r("ZodUnion", (e, t) => {
	bn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => pi(e, t, n, r), e.options = t.options;
});
function Ea(e, t) {
	return new Ta({
		type: "union",
		options: e,
		...w(t)
	});
}
var Da = /* @__PURE__ */ r("ZodDiscriminatedUnion", (e, t) => {
	Ta.init(e, t), xn.init(e, t);
});
function Oa(e, t, n) {
	return new Da({
		type: "union",
		options: t,
		discriminator: e,
		...w(n)
	});
}
var ka = /* @__PURE__ */ r("ZodIntersection", (e, t) => {
	Sn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => mi(e, t, n, r);
});
function Aa(e, t) {
	return new ka({
		type: "intersection",
		left: e,
		right: t
	});
}
var ja = /* @__PURE__ */ r("ZodRecord", (e, t) => {
	Tn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => hi(e, t, n, r), e.keyType = t.keyType, e.valueType = t.valueType;
});
function Ma(e, t, n) {
	return !t || !t._zod ? new ja({
		type: "record",
		keyType: N(),
		valueType: e,
		...w(t)
	}) : new ja({
		type: "record",
		keyType: e,
		valueType: t,
		...w(n)
	});
}
var Na = /* @__PURE__ */ r("ZodEnum", (e, t) => {
	En.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => si(e, t, n, r), e.enum = t.entries, e.options = Object.values(t.entries);
	let n = new Set(Object.keys(t.entries));
	e.extract = (e, r) => {
		let i = {};
		for (let r of e) if (n.has(r)) i[r] = t.entries[r];
		else throw Error(`Key ${r} not found in enum`);
		return new Na({
			...t,
			checks: [],
			...w(r),
			entries: i
		});
	}, e.exclude = (e, r) => {
		let i = { ...t.entries };
		for (let t of e) if (n.has(t)) delete i[t];
		else throw Error(`Key ${t} not found in enum`);
		return new Na({
			...t,
			checks: [],
			...w(r),
			entries: i
		});
	};
});
function Pa(e, t) {
	return new Na({
		type: "enum",
		entries: Array.isArray(e) ? Object.fromEntries(e.map((e) => [e, e])) : e,
		...w(t)
	});
}
var Fa = /* @__PURE__ */ r("ZodLiteral", (e, t) => {
	Dn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => ci(e, t, n, r), e.values = new Set(t.values), Object.defineProperty(e, "value", { get() {
		if (t.values.length > 1) throw Error("This schema contains multiple valid literal values. Use `.values` instead.");
		return t.values[0];
	} });
});
function Ia(e, t) {
	return new Fa({
		type: "literal",
		values: Array.isArray(e) ? e : [e],
		...w(t)
	});
}
var La = /* @__PURE__ */ r("ZodTransform", (e, t) => {
	On.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => ui(e, t, n, r), e._zod.parse = (n, r) => {
		if (r.direction === "backward") throw new a(e.constructor.name);
		n.addIssue = (r) => {
			if (typeof r == "string") n.issues.push(ve(r, n.value, t));
			else {
				let t = r;
				t.fatal && (t.continue = !1), t.code ??= "custom", t.input ??= n.value, t.inst ??= e, n.issues.push(ve(t));
			}
		};
		let i = t.transform(n.value, n);
		return i instanceof Promise ? i.then((e) => (n.value = e, n.fallback = !0, n)) : (n.value = i, n.fallback = !0, n);
	};
});
function Ra(e) {
	return new La({
		type: "transform",
		transform: e
	});
}
var za = /* @__PURE__ */ r("ZodOptional", (e, t) => {
	An.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => Ci(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function Ba(e) {
	return new za({
		type: "optional",
		innerType: e
	});
}
var Va = /* @__PURE__ */ r("ZodExactOptional", (e, t) => {
	jn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => Ci(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function Ha(e) {
	return new Va({
		type: "optional",
		innerType: e
	});
}
var Ua = /* @__PURE__ */ r("ZodNullable", (e, t) => {
	Mn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => gi(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function Wa(e) {
	return new Ua({
		type: "nullable",
		innerType: e
	});
}
var Ga = /* @__PURE__ */ r("ZodDefault", (e, t) => {
	Nn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => vi(e, t, n, r), e.unwrap = () => e._zod.def.innerType, e.removeDefault = e.unwrap;
});
function Ka(e, t) {
	return new Ga({
		type: "default",
		innerType: e,
		get defaultValue() {
			return typeof t == "function" ? t() : te(t);
		}
	});
}
var qa = /* @__PURE__ */ r("ZodPrefault", (e, t) => {
	Fn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => yi(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function Ja(e, t) {
	return new qa({
		type: "prefault",
		innerType: e,
		get defaultValue() {
			return typeof t == "function" ? t() : te(t);
		}
	});
}
var Ya = /* @__PURE__ */ r("ZodNonOptional", (e, t) => {
	In.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => _i(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function Xa(e, t) {
	return new Ya({
		type: "nonoptional",
		innerType: e,
		...w(t)
	});
}
var Za = /* @__PURE__ */ r("ZodCatch", (e, t) => {
	Rn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => bi(e, t, n, r), e.unwrap = () => e._zod.def.innerType, e.removeCatch = e.unwrap;
});
function Qa(e, t) {
	return new Za({
		type: "catch",
		innerType: e,
		catchValue: typeof t == "function" ? t : () => t
	});
}
var $a = /* @__PURE__ */ r("ZodPipe", (e, t) => {
	zn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => xi(e, t, n, r), e.in = t.in, e.out = t.out;
});
function eo(e, t) {
	return new $a({
		type: "pipe",
		in: e,
		out: t
	});
}
var to = /* @__PURE__ */ r("ZodReadonly", (e, t) => {
	Vn.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => Si(e, t, n, r), e.unwrap = () => e._zod.def.innerType;
});
function no(e) {
	return new to({
		type: "readonly",
		innerType: e
	});
}
var ro = /* @__PURE__ */ r("ZodCustom", (e, t) => {
	Un.init(e, t), M.init(e, t), e._zod.processJSONSchema = (t, n, r) => li(e, t, n, r);
});
function io(e, t = {}) {
	return /* @__PURE__ */ Jr(ro, e, t);
}
function ao(e, t) {
	return /* @__PURE__ */ Yr(e, t);
}
//#endregion
//#region node_modules/.pnpm/@agent-ix+workflow-core@0.1.5/node_modules/@agent-ix/workflow-core/dist/index.js
var oo = Pa([
	"auto",
	"hitl",
	"full-auto"
]), so = I({
	name: N().min(1),
	terminal: va().default(!1),
	hint: N().optional()
}).strict(), co = I({
	from: N().min(1),
	to: N().min(1),
	invariants: Ca(N().min(1)).default([]),
	defaultGate: oo.default("auto")
}).strict(), lo = /^[^/\0][^\0]*$/;
function uo(e) {
	return lo.test(e) ? !e.split("/").includes("..") : !1;
}
var fo = Pa([
	"text",
	"list<text>",
	"enum",
	"bool",
	"int"
]), po = I({
	key: N().min(1),
	prompt: N().min(1),
	type: fo,
	required: va(),
	options: Ca(N().min(1)).optional(),
	minItems: ma().int().nonnegative().optional(),
	guidance: N().optional(),
	followUpIf: Pa(["nonEmpty", "empty"]).optional()
}).strict().superRefine((e, t) => {
	e.type === "enum" && (!e.options || e.options.length === 0) && t.addIssue({
		code: "custom",
		path: ["options"],
		message: "interview_enum_options_missing"
	});
}), mo = N().regex(/^(all_required|min_count:\d+|custom_invariant:.+)$/).default("all_required"), ho = I({
	itemType: N().min(1),
	completenessRule: mo,
	questions: Ca(po).min(1)
}).strict().superRefine((e, t) => {
	let n = /* @__PURE__ */ new Set();
	for (let [r, i] of e.questions.entries()) n.has(i.key) && t.addIssue({
		code: "custom",
		path: [
			"questions",
			r,
			"key"
		],
		message: "interview_key_duplicate"
	}), n.add(i.key);
}), go = I({
	source: N().min(1),
	target: N().min(1),
	variant: N().min(1).optional(),
	frontmatterSchema: N().min(1).optional(),
	phase: N().min(1).optional()
}).strict().superRefine((e, t) => {
	uo(e.source) || t.addIssue({
		code: "custom",
		path: ["source"],
		message: "template_path_invalid"
	}), uo(e.target) || t.addIssue({
		code: "custom",
		path: ["target"],
		message: "template_path_invalid"
	});
}), _o = Oa("command", [
	I({
		command: Ia("advance"),
		to: N().min(1)
	}).strict(),
	I({
		command: Ia("add-item"),
		type: N().min(1),
		item: F()
	}).strict(),
	I({
		command: Ia("update-item"),
		type: N().min(1),
		itemId: N().min(1),
		patch: F()
	}).strict(),
	I({
		command: Ia("link-items"),
		link: F()
	}).strict(),
	I({
		command: Ia("record-answers"),
		interviewId: N().min(1),
		answers: Ma(N(), F()),
		merge: va().optional()
	}).strict()
]), vo = I({
	description: N().optional(),
	steps: Ca(_o).min(1)
}).strict();
I({
	name: N().min(1),
	version: N().min(1),
	description: N().optional(),
	initialPhase: N().min(1),
	phases: Ca(so).min(1),
	transitions: Ca(co).default([]),
	itemSchemas: Ma(N(), F()).default({}),
	linkSchemas: Ma(N(), F()).default({}),
	artifactTemplates: Ma(N(), go).optional(),
	interviews: Ma(N(), ho).optional(),
	recipes: Ma(N().min(1), vo).optional()
}).strict().superRefine((e, t) => {
	let n = new Set(e.phases.map((e) => e.name));
	n.has(e.initialPhase) || t.addIssue({
		code: "custom",
		path: ["initialPhase"],
		message: "initialPhase must reference a declared phase"
	});
	for (let [r, i] of e.transitions.entries()) n.has(i.from) || t.addIssue({
		code: "custom",
		path: [
			"transitions",
			r,
			"from"
		],
		message: "transition.from must reference a declared phase"
	}), n.has(i.to) || t.addIssue({
		code: "custom",
		path: [
			"transitions",
			r,
			"to"
		],
		message: "transition.to must reference a declared phase"
	});
	if (e.interviews) for (let [n, r] of Object.entries(e.interviews)) r.itemType in e.itemSchemas || t.addIssue({
		code: "custom",
		path: [
			"interviews",
			n,
			"itemType"
		],
		message: `interview_item_type_unknown: ${r.itemType}`
	});
	if (e.artifactTemplates) {
		let r = /* @__PURE__ */ new Map();
		for (let [i, a] of Object.entries(e.artifactTemplates)) if (a.phase && !n.has(a.phase) && t.addIssue({
			code: "custom",
			path: [
				"artifactTemplates",
				i,
				"phase"
			],
			message: "artifactTemplates.phase must reference a declared phase"
		}), !a.target.includes("${")) {
			let e = r.get(a.target);
			e ? t.addIssue({
				code: "custom",
				path: [
					"artifactTemplates",
					i,
					"target"
				],
				message: `template_target_duplicate: shares target with ${e}`
			}) : r.set(a.target, i);
		}
	}
}), "0".repeat(64);
var yo = Symbol.for("yaml.alias"), bo = Symbol.for("yaml.document"), xo = Symbol.for("yaml.map"), So = Symbol.for("yaml.pair"), L = Symbol.for("yaml.scalar"), Co = Symbol.for("yaml.seq"), R = Symbol.for("yaml.node.type"), wo = (e) => !!e && typeof e == "object" && e[R] === yo, To = (e) => !!e && typeof e == "object" && e[R] === bo, Eo = (e) => !!e && typeof e == "object" && e[R] === xo, z = (e) => !!e && typeof e == "object" && e[R] === So, B = (e) => !!e && typeof e == "object" && e[R] === L, Do = (e) => !!e && typeof e == "object" && e[R] === Co;
function V(e) {
	if (e && typeof e == "object") switch (e[R]) {
		case xo:
		case Co: return !0;
	}
	return !1;
}
function H(e) {
	if (e && typeof e == "object") switch (e[R]) {
		case yo:
		case xo:
		case L:
		case Co: return !0;
	}
	return !1;
}
var Oo = (e) => (B(e) || V(e)) && !!e.anchor, U = Symbol("break visit"), ko = Symbol("skip children"), W = Symbol("remove node");
function Ao(e, t) {
	let n = Po(t);
	To(e) ? jo(null, e.contents, n, Object.freeze([e])) === W && (e.contents = null) : jo(null, e, n, Object.freeze([]));
}
Ao.BREAK = U, Ao.SKIP = ko, Ao.REMOVE = W;
function jo(e, t, n, r) {
	let i = Fo(e, t, n, r);
	if (H(i) || z(i)) return Io(e, r, i), jo(e, i, n, r);
	if (typeof i != "symbol") {
		if (V(t)) {
			r = Object.freeze(r.concat(t));
			for (let e = 0; e < t.items.length; ++e) {
				let i = jo(e, t.items[e], n, r);
				if (typeof i == "number") e = i - 1;
				else if (i === U) return U;
				else i === W && (t.items.splice(e, 1), --e);
			}
		} else if (z(t)) {
			r = Object.freeze(r.concat(t));
			let e = jo("key", t.key, n, r);
			if (e === U) return U;
			e === W && (t.key = null);
			let i = jo("value", t.value, n, r);
			if (i === U) return U;
			i === W && (t.value = null);
		}
	}
	return i;
}
async function Mo(e, t) {
	let n = Po(t);
	To(e) ? await No(null, e.contents, n, Object.freeze([e])) === W && (e.contents = null) : await No(null, e, n, Object.freeze([]));
}
Mo.BREAK = U, Mo.SKIP = ko, Mo.REMOVE = W;
async function No(e, t, n, r) {
	let i = await Fo(e, t, n, r);
	if (H(i) || z(i)) return Io(e, r, i), No(e, i, n, r);
	if (typeof i != "symbol") {
		if (V(t)) {
			r = Object.freeze(r.concat(t));
			for (let e = 0; e < t.items.length; ++e) {
				let i = await No(e, t.items[e], n, r);
				if (typeof i == "number") e = i - 1;
				else if (i === U) return U;
				else i === W && (t.items.splice(e, 1), --e);
			}
		} else if (z(t)) {
			r = Object.freeze(r.concat(t));
			let e = await No("key", t.key, n, r);
			if (e === U) return U;
			e === W && (t.key = null);
			let i = await No("value", t.value, n, r);
			if (i === U) return U;
			i === W && (t.value = null);
		}
	}
	return i;
}
function Po(e) {
	return typeof e == "object" && (e.Collection || e.Node || e.Value) ? Object.assign({
		Alias: e.Node,
		Map: e.Node,
		Scalar: e.Node,
		Seq: e.Node
	}, e.Value && {
		Map: e.Value,
		Scalar: e.Value,
		Seq: e.Value
	}, e.Collection && {
		Map: e.Collection,
		Seq: e.Collection
	}, e) : e;
}
function Fo(e, t, n, r) {
	if (typeof n == "function") return n(e, t, r);
	if (Eo(t)) return n.Map?.(e, t, r);
	if (Do(t)) return n.Seq?.(e, t, r);
	if (z(t)) return n.Pair?.(e, t, r);
	if (B(t)) return n.Scalar?.(e, t, r);
	if (wo(t)) return n.Alias?.(e, t, r);
}
function Io(e, t, n) {
	let r = t[t.length - 1];
	if (V(r)) r.items[e] = n;
	else if (z(r)) e === "key" ? r.key = n : r.value = n;
	else if (To(r)) r.contents = n;
	else {
		let e = wo(r) ? "alias" : "scalar";
		throw Error(`Cannot replace node with ${e} parent`);
	}
}
var Lo = {
	"!": "%21",
	",": "%2C",
	"[": "%5B",
	"]": "%5D",
	"{": "%7B",
	"}": "%7D"
}, Ro = (e) => e.replace(/[!,[\]{}]/g, (e) => Lo[e]), zo = class e {
	constructor(t, n) {
		this.docStart = null, this.docEnd = !1, this.yaml = Object.assign({}, e.defaultYaml, t), this.tags = Object.assign({}, e.defaultTags, n);
	}
	clone() {
		let t = new e(this.yaml, this.tags);
		return t.docStart = this.docStart, t;
	}
	atDocument() {
		let t = new e(this.yaml, this.tags);
		switch (this.yaml.version) {
			case "1.1":
				this.atNextDocument = !0;
				break;
			case "1.2":
				this.atNextDocument = !1, this.yaml = {
					explicit: e.defaultYaml.explicit,
					version: "1.2"
				}, this.tags = Object.assign({}, e.defaultTags);
				break;
		}
		return t;
	}
	add(t, n) {
		this.atNextDocument &&= (this.yaml = {
			explicit: e.defaultYaml.explicit,
			version: "1.1"
		}, this.tags = Object.assign({}, e.defaultTags), !1);
		let r = t.trim().split(/[ \t]+/), i = r.shift();
		switch (i) {
			case "%TAG": {
				if (r.length !== 2 && (n(0, "%TAG directive should contain exactly two parts"), r.length < 2)) return !1;
				let [e, t] = r;
				return this.tags[e] = t, !0;
			}
			case "%YAML": {
				if (this.yaml.explicit = !0, r.length !== 1) return n(0, "%YAML directive should contain exactly one part"), !1;
				let [e] = r;
				if (e === "1.1" || e === "1.2") return this.yaml.version = e, !0;
				{
					let t = /^\d+\.\d+$/.test(e);
					return n(6, `Unsupported YAML version ${e}`, t), !1;
				}
			}
			default: return n(0, `Unknown directive ${i}`, !0), !1;
		}
	}
	tagName(e, t) {
		if (e === "!") return "!";
		if (e[0] !== "!") return t(`Not a valid tag: ${e}`), null;
		if (e[1] === "<") {
			let n = e.slice(2, -1);
			return n === "!" || n === "!!" ? (t(`Verbatim tags aren't resolved, so ${e} is invalid.`), null) : (e[e.length - 1] !== ">" && t("Verbatim tags must end with a >"), n);
		}
		let [, n, r] = e.match(/^(.*!)([^!]*)$/s);
		r || t(`The ${e} tag has no suffix`);
		let i = this.tags[n];
		if (i) try {
			return i + decodeURIComponent(r);
		} catch (e) {
			return t(String(e)), null;
		}
		return n === "!" ? e : (t(`Could not resolve tag: ${e}`), null);
	}
	tagString(e) {
		for (let [t, n] of Object.entries(this.tags)) if (e.startsWith(n)) return t + Ro(e.substring(n.length));
		return e[0] === "!" ? e : `!<${e}>`;
	}
	toString(e) {
		let t = this.yaml.explicit ? [`%YAML ${this.yaml.version || "1.2"}`] : [], n = Object.entries(this.tags), r;
		if (e && n.length > 0 && H(e.contents)) {
			let t = {};
			Ao(e.contents, (e, n) => {
				H(n) && n.tag && (t[n.tag] = !0);
			}), r = Object.keys(t);
		} else r = [];
		for (let [i, a] of n) i === "!!" && a === "tag:yaml.org,2002:" || (!e || r.some((e) => e.startsWith(a))) && t.push(`%TAG ${i} ${a}`);
		return t.join("\n");
	}
};
zo.defaultYaml = {
	explicit: !1,
	version: "1.2"
}, zo.defaultTags = { "!!": "tag:yaml.org,2002:" };
function Bo(e) {
	if (/[\x00-\x19\s,[\]{}]/.test(e)) {
		let t = `Anchor must not contain whitespace or control characters: ${JSON.stringify(e)}`;
		throw Error(t);
	}
	return !0;
}
function Vo(e) {
	let t = /* @__PURE__ */ new Set();
	return Ao(e, { Value(e, n) {
		n.anchor && t.add(n.anchor);
	} }), t;
}
function Ho(e, t) {
	for (let n = 1;; ++n) {
		let r = `${e}${n}`;
		if (!t.has(r)) return r;
	}
}
function Uo(e, t) {
	let n = [], r = /* @__PURE__ */ new Map(), i = null;
	return {
		onAnchor: (r) => {
			n.push(r), i ??= Vo(e);
			let a = Ho(t, i);
			return i.add(a), a;
		},
		setAnchors: () => {
			for (let e of n) {
				let t = r.get(e);
				if (typeof t == "object" && t.anchor && (B(t.node) || V(t.node))) t.node.anchor = t.anchor;
				else {
					let t = /* @__PURE__ */ Error("Failed to resolve repeated object (this should not happen)");
					throw t.source = e, t;
				}
			}
		},
		sourceObjects: r
	};
}
function Wo(e, t, n, r) {
	if (r && typeof r == "object") if (Array.isArray(r)) for (let t = 0, n = r.length; t < n; ++t) {
		let n = r[t], i = Wo(e, r, String(t), n);
		i === void 0 ? delete r[t] : i !== n && (r[t] = i);
	}
	else if (r instanceof Map) for (let t of Array.from(r.keys())) {
		let n = r.get(t), i = Wo(e, r, t, n);
		i === void 0 ? r.delete(t) : i !== n && r.set(t, i);
	}
	else if (r instanceof Set) for (let t of Array.from(r)) {
		let n = Wo(e, r, t, t);
		n === void 0 ? r.delete(t) : n !== t && (r.delete(t), r.add(n));
	}
	else for (let [t, n] of Object.entries(r)) {
		let i = Wo(e, r, t, n);
		i === void 0 ? delete r[t] : i !== n && (r[t] = i);
	}
	return e.call(t, n, r);
}
function G(e, t, n) {
	if (Array.isArray(e)) return e.map((e, t) => G(e, String(t), n));
	if (e && typeof e.toJSON == "function") {
		if (!n || !Oo(e)) return e.toJSON(t, n);
		let r = {
			aliasCount: 0,
			count: 1,
			res: void 0
		};
		n.anchors.set(e, r), n.onCreate = (e) => {
			r.res = e, delete n.onCreate;
		};
		let i = e.toJSON(t, n);
		return n.onCreate && n.onCreate(i), i;
	}
	return typeof e == "bigint" && !n?.keep ? Number(e) : e;
}
var Go = class {
	constructor(e) {
		Object.defineProperty(this, R, { value: e });
	}
	clone() {
		let e = Object.create(Object.getPrototypeOf(this), Object.getOwnPropertyDescriptors(this));
		return this.range && (e.range = this.range.slice()), e;
	}
	toJS(e, { mapAsMap: t, maxAliasCount: n, onAnchor: r, reviver: i } = {}) {
		if (!To(e)) throw TypeError("A document argument is required");
		let a = {
			anchors: /* @__PURE__ */ new Map(),
			doc: e,
			keep: !0,
			mapAsMap: t === !0,
			mapKeyWarned: !1,
			maxAliasCount: typeof n == "number" ? n : 100
		}, o = G(this, "", a);
		if (typeof r == "function") for (let { count: e, res: t } of a.anchors.values()) r(t, e);
		return typeof i == "function" ? Wo(i, { "": o }, "", o) : o;
	}
}, Ko = class extends Go {
	constructor(e) {
		super(yo), this.source = e, Object.defineProperty(this, "tag", { set() {
			throw Error("Alias nodes cannot have tags");
		} });
	}
	resolve(e, t) {
		if (t?.maxAliasCount === 0) throw ReferenceError("Alias resolution is disabled");
		let n;
		t?.aliasResolveCache ? n = t.aliasResolveCache : (n = [], Ao(e, { Node: (e, t) => {
			(wo(t) || Oo(t)) && n.push(t);
		} }), t && (t.aliasResolveCache = n));
		let r;
		for (let e of n) {
			if (e === this) break;
			e.anchor === this.source && (r = e);
		}
		return r;
	}
	toJSON(e, t) {
		if (!t) return { source: this.source };
		let { anchors: n, doc: r, maxAliasCount: i } = t, a = this.resolve(r, t);
		if (!a) {
			let e = `Unresolved alias (the anchor must be set before the alias): ${this.source}`;
			throw ReferenceError(e);
		}
		let o = n.get(a);
		/* istanbul ignore if */
		if (o ||= (G(a, null, t), n.get(a)), o?.res === void 0) throw ReferenceError("This should not happen: Alias anchor was not resolved?");
		if (i >= 0 && (o.count += 1, o.aliasCount === 0 && (o.aliasCount = qo(r, a, n)), o.count * o.aliasCount > i)) throw ReferenceError("Excessive alias count indicates a resource exhaustion attack");
		return o.res;
	}
	toString(e, t, n) {
		let r = `*${this.source}`;
		if (e) {
			if (Bo(this.source), e.options.verifyAliasOrder && !e.anchors.has(this.source)) {
				let e = `Unresolved alias (the anchor must be set before the alias): ${this.source}`;
				throw Error(e);
			}
			if (e.implicitKey) return `${r} `;
		}
		return r;
	}
};
function qo(e, t, n) {
	if (wo(t)) {
		let r = t.resolve(e), i = n && r && n.get(r);
		return i ? i.count * i.aliasCount : 0;
	} else if (V(t)) {
		let r = 0;
		for (let i of t.items) {
			let t = qo(e, i, n);
			t > r && (r = t);
		}
		return r;
	} else if (z(t)) {
		let r = qo(e, t.key, n), i = qo(e, t.value, n);
		return Math.max(r, i);
	}
	return 1;
}
var Jo = (e) => !e || typeof e != "function" && typeof e != "object", K = class extends Go {
	constructor(e) {
		super(L), this.value = e;
	}
	toJSON(e, t) {
		return t?.keep ? this.value : G(this.value, e, t);
	}
	toString() {
		return String(this.value);
	}
};
K.BLOCK_FOLDED = "BLOCK_FOLDED", K.BLOCK_LITERAL = "BLOCK_LITERAL", K.PLAIN = "PLAIN", K.QUOTE_DOUBLE = "QUOTE_DOUBLE", K.QUOTE_SINGLE = "QUOTE_SINGLE";
var Yo = "tag:yaml.org,2002:";
function Xo(e, t, n) {
	if (t) {
		let e = n.filter((e) => e.tag === t), r = e.find((e) => !e.format) ?? e[0];
		if (!r) throw Error(`Tag ${t} not found`);
		return r;
	}
	return n.find((t) => t.identify?.(e) && !t.format);
}
function Zo(e, t, n) {
	if (To(e) && (e = e.contents), H(e)) return e;
	if (z(e)) {
		let t = n.schema[xo].createNode?.(n.schema, null, n);
		return t.items.push(e), t;
	}
	(e instanceof String || e instanceof Number || e instanceof Boolean || typeof BigInt < "u" && e instanceof BigInt) && (e = e.valueOf());
	let { aliasDuplicateObjects: r, onAnchor: i, onTagObj: a, schema: o, sourceObjects: s } = n, c;
	if (r && e && typeof e == "object") {
		if (c = s.get(e), c) return c.anchor ??= i(e), new Ko(c.anchor);
		c = {
			anchor: null,
			node: null
		}, s.set(e, c);
	}
	t?.startsWith("!!") && (t = Yo + t.slice(2));
	let l = Xo(e, t, o.tags);
	if (!l) {
		if (e && typeof e.toJSON == "function" && (e = e.toJSON()), !e || typeof e != "object") {
			let t = new K(e);
			return c && (c.node = t), t;
		}
		l = e instanceof Map ? o[xo] : Symbol.iterator in Object(e) ? o[Co] : o[xo];
	}
	a && (a(l), delete n.onTagObj);
	let u = l?.createNode ? l.createNode(n.schema, e, n) : typeof l?.nodeClass?.from == "function" ? l.nodeClass.from(n.schema, e, n) : new K(e);
	return t ? u.tag = t : l.default || (u.tag = l.tag), c && (c.node = u), u;
}
function Qo(e, t, n) {
	let r = n;
	for (let e = t.length - 1; e >= 0; --e) {
		let n = t[e];
		if (typeof n == "number" && Number.isInteger(n) && n >= 0) {
			let e = [];
			e[n] = r, r = e;
		} else r = new Map([[n, r]]);
	}
	return Zo(r, void 0, {
		aliasDuplicateObjects: !1,
		keepUndefined: !1,
		onAnchor: () => {
			throw Error("This should not happen, please report a bug.");
		},
		schema: e,
		sourceObjects: /* @__PURE__ */ new Map()
	});
}
var $o = (e) => e == null || typeof e == "object" && !!e[Symbol.iterator]().next().done, es = class extends Go {
	constructor(e, t) {
		super(e), Object.defineProperty(this, "schema", {
			value: t,
			configurable: !0,
			enumerable: !1,
			writable: !0
		});
	}
	clone(e) {
		let t = Object.create(Object.getPrototypeOf(this), Object.getOwnPropertyDescriptors(this));
		return e && (t.schema = e), t.items = t.items.map((t) => H(t) || z(t) ? t.clone(e) : t), this.range && (t.range = this.range.slice()), t;
	}
	addIn(e, t) {
		if ($o(e)) this.add(t);
		else {
			let [n, ...r] = e, i = this.get(n, !0);
			if (V(i)) i.addIn(r, t);
			else if (i === void 0 && this.schema) this.set(n, Qo(this.schema, r, t));
			else throw Error(`Expected YAML collection at ${n}. Remaining path: ${r}`);
		}
	}
	deleteIn(e) {
		let [t, ...n] = e;
		if (n.length === 0) return this.delete(t);
		let r = this.get(t, !0);
		if (V(r)) return r.deleteIn(n);
		throw Error(`Expected YAML collection at ${t}. Remaining path: ${n}`);
	}
	getIn(e, t) {
		let [n, ...r] = e, i = this.get(n, !0);
		return r.length === 0 ? !t && B(i) ? i.value : i : V(i) ? i.getIn(r, t) : void 0;
	}
	hasAllNullValues(e) {
		return this.items.every((t) => {
			if (!z(t)) return !1;
			let n = t.value;
			return n == null || e && B(n) && n.value == null && !n.commentBefore && !n.comment && !n.tag;
		});
	}
	hasIn(e) {
		let [t, ...n] = e;
		if (n.length === 0) return this.has(t);
		let r = this.get(t, !0);
		return V(r) ? r.hasIn(n) : !1;
	}
	setIn(e, t) {
		let [n, ...r] = e;
		if (r.length === 0) this.set(n, t);
		else {
			let e = this.get(n, !0);
			if (V(e)) e.setIn(r, t);
			else if (e === void 0 && this.schema) this.set(n, Qo(this.schema, r, t));
			else throw Error(`Expected YAML collection at ${n}. Remaining path: ${r}`);
		}
	}
}, ts = (e) => e.replace(/^(?!$)(?: $)?/gm, "#");
function q(e, t) {
	return /^\n+$/.test(e) ? e.substring(1) : t ? e.replace(/^(?! *$)/gm, t) : e;
}
var ns = (e, t, n) => e.endsWith("\n") ? q(n, t) : n.includes("\n") ? "\n" + q(n, t) : (e.endsWith(" ") ? "" : " ") + n, rs = "flow", is = "block", as = "quoted";
function os(e, t, n = "flow", { indentAtStart: r, lineWidth: i = 80, minContentWidth: a = 20, onFold: o, onOverflow: s } = {}) {
	if (!i || i < 0) return e;
	i < a && (a = 0);
	let c = Math.max(1 + a, 1 + i - t.length);
	if (e.length <= c) return e;
	let l = [], u = {}, d = i - t.length;
	typeof r == "number" && (r > i - Math.max(2, a) ? l.push(0) : d = i - r);
	let f, p, m = !1, h = -1, g = -1, _ = -1;
	n === "block" && (h = ss(e, h, t.length), h !== -1 && (d = h + c));
	for (let r; r = e[h += 1];) {
		if (n === "quoted" && r === "\\") {
			switch (g = h, e[h + 1]) {
				case "x":
					h += 3;
					break;
				case "u":
					h += 5;
					break;
				case "U":
					h += 9;
					break;
				default: h += 1;
			}
			_ = h;
		}
		if (r === "\n") n === "block" && (h = ss(e, h, t.length)), d = h + t.length + c, f = void 0;
		else {
			if (r === " " && p && p !== " " && p !== "\n" && p !== "	") {
				let t = e[h + 1];
				t && t !== " " && t !== "\n" && t !== "	" && (f = h);
			}
			if (h >= d) if (f) l.push(f), d = f + c, f = void 0;
			else if (n === "quoted") {
				for (; p === " " || p === "	";) p = r, r = e[h += 1], m = !0;
				let t = h > _ + 1 ? h - 2 : g - 1;
				if (u[t]) return e;
				l.push(t), u[t] = !0, d = t + c, f = void 0;
			} else m = !0;
		}
		p = r;
	}
	if (m && s && s(), l.length === 0) return e;
	o && o();
	let v = e.slice(0, l[0]);
	for (let r = 0; r < l.length; ++r) {
		let i = l[r], a = l[r + 1] || e.length;
		i === 0 ? v = `\n${t}${e.slice(0, a)}` : (n === "quoted" && u[i] && (v += `${e[i]}\\`), v += `\n${t}${e.slice(i + 1, a)}`);
	}
	return v;
}
function ss(e, t, n) {
	let r = t, i = t + 1, a = e[i];
	for (; a === " " || a === "	";) if (t < i + n) a = e[++t];
	else {
		do
			a = e[++t];
		while (a && a !== "\n");
		r = t, i = t + 1, a = e[i];
	}
	return r;
}
var cs = (e, t) => ({
	indentAtStart: t ? e.indent.length : e.indentAtStart,
	lineWidth: e.options.lineWidth,
	minContentWidth: e.options.minContentWidth
}), ls = (e) => /^(%|---|\.\.\.)/m.test(e);
function us(e, t, n) {
	if (!t || t < 0) return !1;
	let r = t - n, i = e.length;
	if (i <= r) return !1;
	for (let t = 0, n = 0; t < i; ++t) if (e[t] === "\n") {
		if (t - n > r) return !0;
		if (n = t + 1, i - n <= r) return !1;
	}
	return !0;
}
function ds(e, t) {
	let n = JSON.stringify(e);
	if (t.options.doubleQuotedAsJSON) return n;
	let { implicitKey: r } = t, i = t.options.doubleQuotedMinMultiLineLength, a = t.indent || (ls(e) ? "  " : ""), o = "", s = 0;
	for (let e = 0, t = n[e]; t; t = n[++e]) if (t === " " && n[e + 1] === "\\" && n[e + 2] === "n" && (o += n.slice(s, e) + "\\ ", e += 1, s = e, t = "\\"), t === "\\") switch (n[e + 1]) {
		case "u":
			{
				o += n.slice(s, e);
				let t = n.substr(e + 2, 4);
				switch (t) {
					case "0000":
						o += "\\0";
						break;
					case "0007":
						o += "\\a";
						break;
					case "000b":
						o += "\\v";
						break;
					case "001b":
						o += "\\e";
						break;
					case "0085":
						o += "\\N";
						break;
					case "00a0":
						o += "\\_";
						break;
					case "2028":
						o += "\\L";
						break;
					case "2029":
						o += "\\P";
						break;
					default: t.substr(0, 2) === "00" ? o += "\\x" + t.substr(2) : o += n.substr(e, 6);
				}
				e += 5, s = e + 1;
			}
			break;
		case "n":
			if (r || n[e + 2] === "\"" || n.length < i) e += 1;
			else {
				for (o += n.slice(s, e) + "\n\n"; n[e + 2] === "\\" && n[e + 3] === "n" && n[e + 4] !== "\"";) o += "\n", e += 2;
				o += a, n[e + 2] === " " && (o += "\\"), e += 1, s = e + 1;
			}
			break;
		default: e += 1;
	}
	return o = s ? o + n.slice(s) : n, r ? o : os(o, a, as, cs(t, !1));
}
function fs(e, t) {
	if (t.options.singleQuote === !1 || t.implicitKey && e.includes("\n") || /[ \t]\n|\n[ \t]/.test(e)) return ds(e, t);
	let n = t.indent || (ls(e) ? "  " : ""), r = "'" + e.replace(/'/g, "''").replace(/\n+/g, `$&\n${n}`) + "'";
	return t.implicitKey ? r : os(r, n, rs, cs(t, !1));
}
function ps(e, t) {
	let { singleQuote: n } = t.options, r;
	if (n === !1) r = ds;
	else {
		let t = e.includes("\""), i = e.includes("'");
		r = t && !i ? fs : i && !t ? ds : n ? fs : ds;
	}
	return r(e, t);
}
var ms;
try {
	ms = /* @__PURE__ */ RegExp("(^|(?<!\n))\n+(?!\n|$)", "g");
} catch {
	ms = /\n+(?!\n|$)/g;
}
function hs({ comment: e, type: t, value: n }, r, i, a) {
	let { blockQuote: o, commentString: s, lineWidth: c } = r.options;
	if (!o || /\n[\t ]+$/.test(n)) return ps(n, r);
	let l = r.indent || (r.forceBlockIndent || ls(n) ? "  " : ""), u = o === "literal" ? !0 : o === "folded" || t === K.BLOCK_FOLDED ? !1 : t === K.BLOCK_LITERAL ? !0 : !us(n, c, l.length);
	if (!n) return u ? "|\n" : ">\n";
	let d, f;
	for (f = n.length; f > 0; --f) {
		let e = n[f - 1];
		if (e !== "\n" && e !== "	" && e !== " ") break;
	}
	let p = n.substring(f), m = p.indexOf("\n");
	m === -1 ? d = "-" : n === p || m !== p.length - 1 ? (d = "+", a && a()) : d = "", p &&= (n = n.slice(0, -p.length), p[p.length - 1] === "\n" && (p = p.slice(0, -1)), p.replace(ms, `$&${l}`));
	let h = !1, g, _ = -1;
	for (g = 0; g < n.length; ++g) {
		let e = n[g];
		if (e === " ") h = !0;
		else if (e === "\n") _ = g;
		else break;
	}
	let v = n.substring(0, _ < g ? _ + 1 : g);
	v &&= (n = n.substring(v.length), v.replace(/\n+/g, `$&${l}`));
	let y = (h ? l ? "2" : "1" : "") + d;
	if (e && (y += " " + s(e.replace(/ ?[\r\n]+/g, " ")), i && i()), !u) {
		let e = n.replace(/\n+/g, "\n$&").replace(/(?:^|\n)([\t ].*)(?:([\n\t ]*)\n(?![\n\t ]))?/g, "$1$2").replace(/\n+/g, `$&${l}`), i = !1, a = cs(r, !0);
		o !== "folded" && t !== K.BLOCK_FOLDED && (a.onOverflow = () => {
			i = !0;
		});
		let s = os(`${v}${e}${p}`, l, is, a);
		if (!i) return `>${y}\n${l}${s}`;
	}
	return n = n.replace(/\n+/g, `$&${l}`), `|${y}\n${l}${v}${n}${p}`;
}
function gs(e, t, n, r) {
	let { type: i, value: a } = e, { actualString: o, implicitKey: s, indent: c, indentStep: l, inFlow: u } = t;
	if (s && a.includes("\n") || u && /[[\]{},]/.test(a)) return ps(a, t);
	if (/^[\n\t ,[\]{}#&*!|>'"%@`]|^[?-]$|^[?-][ \t]|[\n:][ \t]|[ \t]\n|[\n\t ]#|[\n\t :]$/.test(a)) return s || u || !a.includes("\n") ? ps(a, t) : hs(e, t, n, r);
	if (!s && !u && i !== K.PLAIN && a.includes("\n")) return hs(e, t, n, r);
	if (ls(a)) {
		if (c === "") return t.forceBlockIndent = !0, hs(e, t, n, r);
		if (s && c === l) return ps(a, t);
	}
	let d = a.replace(/\n+/g, `$&\n${c}`);
	if (o) {
		let e = (e) => e.default && e.tag !== "tag:yaml.org,2002:str" && e.test?.test(d), { compat: n, tags: r } = t.doc.schema;
		if (r.some(e) || n?.some(e)) return ps(a, t);
	}
	return s ? d : os(d, c, rs, cs(t, !1));
}
function _s(e, t, n, r) {
	let { implicitKey: i, inFlow: a } = t, o = typeof e.value == "string" ? e : Object.assign({}, e, { value: String(e.value) }), { type: s } = e;
	s !== K.QUOTE_DOUBLE && /[\x00-\x08\x0b-\x1f\x7f-\x9f\u{D800}-\u{DFFF}]/u.test(o.value) && (s = K.QUOTE_DOUBLE);
	let c = (e) => {
		switch (e) {
			case K.BLOCK_FOLDED:
			case K.BLOCK_LITERAL: return i || a ? ps(o.value, t) : hs(o, t, n, r);
			case K.QUOTE_DOUBLE: return ds(o.value, t);
			case K.QUOTE_SINGLE: return fs(o.value, t);
			case K.PLAIN: return gs(o, t, n, r);
			default: return null;
		}
	}, l = c(s);
	if (l === null) {
		let { defaultKeyType: e, defaultStringType: n } = t.options, r = i && e || n;
		if (l = c(r), l === null) throw Error(`Unsupported default string type ${r}`);
	}
	return l;
}
function vs(e, t) {
	let n = Object.assign({
		blockQuote: !0,
		commentString: ts,
		defaultKeyType: null,
		defaultStringType: "PLAIN",
		directives: null,
		doubleQuotedAsJSON: !1,
		doubleQuotedMinMultiLineLength: 40,
		falseStr: "false",
		flowCollectionPadding: !0,
		indentSeq: !0,
		lineWidth: 80,
		minContentWidth: 20,
		nullStr: "null",
		simpleKeys: !1,
		singleQuote: null,
		trailingComma: !1,
		trueStr: "true",
		verifyAliasOrder: !0
	}, e.schema.toStringOptions, t), r;
	switch (n.collectionStyle) {
		case "block":
			r = !1;
			break;
		case "flow":
			r = !0;
			break;
		default: r = null;
	}
	return {
		anchors: /* @__PURE__ */ new Set(),
		doc: e,
		flowCollectionPadding: n.flowCollectionPadding ? " " : "",
		indent: "",
		indentStep: typeof n.indent == "number" ? " ".repeat(n.indent) : "  ",
		inFlow: r,
		options: n
	};
}
function ys(e, t) {
	if (t.tag) {
		let n = e.filter((e) => e.tag === t.tag);
		if (n.length > 0) return n.find((e) => e.format === t.format) ?? n[0];
	}
	let n, r;
	if (B(t)) {
		r = t.value;
		let i = e.filter((e) => e.identify?.(r));
		if (i.length > 1) {
			let e = i.filter((e) => e.test);
			e.length > 0 && (i = e);
		}
		n = i.find((e) => e.format === t.format) ?? i.find((e) => !e.format);
	} else r = t, n = e.find((e) => e.nodeClass && r instanceof e.nodeClass);
	if (!n) {
		let e = r?.constructor?.name ?? (r === null ? "null" : typeof r);
		throw Error(`Tag not resolved for ${e} value`);
	}
	return n;
}
function bs(e, t, { anchors: n, doc: r }) {
	if (!r.directives) return "";
	let i = [], a = (B(e) || V(e)) && e.anchor;
	a && Bo(a) && (n.add(a), i.push(`&${a}`));
	let o = e.tag ?? (t.default ? null : t.tag);
	return o && i.push(r.directives.tagString(o)), i.join(" ");
}
function xs(e, t, n, r) {
	if (z(e)) return e.toString(t, n, r);
	if (wo(e)) {
		if (t.doc.directives) return e.toString(t);
		if (t.resolvedAliases?.has(e)) throw TypeError("Cannot stringify circular structure without alias nodes");
		t.resolvedAliases ? t.resolvedAliases.add(e) : t.resolvedAliases = new Set([e]), e = e.resolve(t.doc);
	}
	let i, a = H(e) ? e : t.doc.createNode(e, { onTagObj: (e) => i = e });
	i ??= ys(t.doc.schema.tags, a);
	let o = bs(a, i, t);
	o.length > 0 && (t.indentAtStart = (t.indentAtStart ?? 0) + o.length + 1);
	let s = typeof i.stringify == "function" ? i.stringify(a, t, n, r) : B(a) ? _s(a, t, n, r) : a.toString(t, n, r);
	return o ? B(a) || s[0] === "{" || s[0] === "[" ? `${o} ${s}` : `${o}\n${t.indent}${s}` : s;
}
function Ss({ key: e, value: t }, n, r, i) {
	let { allNullValues: a, doc: o, indent: s, indentStep: c, options: { commentString: l, indentSeq: u, simpleKeys: d } } = n, f = H(e) && e.comment || null;
	if (d) {
		if (f) throw Error("With simple keys, key nodes cannot have comments");
		if (V(e) || !H(e) && typeof e == "object") throw Error("With simple keys, collection cannot be used as a key value");
	}
	let p = !d && (!e || f && t == null && !n.inFlow || V(e) || (B(e) ? e.type === K.BLOCK_FOLDED || e.type === K.BLOCK_LITERAL : typeof e == "object"));
	n = Object.assign({}, n, {
		allNullValues: !1,
		implicitKey: !p && (d || !a),
		indent: s + c
	});
	let m = !1, h = !1, g = xs(e, n, () => m = !0, () => h = !0);
	if (!p && !n.inFlow && g.length > 1024) {
		if (d) throw Error("With simple keys, single line scalar must not span more than 1024 characters");
		p = !0;
	}
	if (n.inFlow) {
		if (a || t == null) return m && r && r(), g === "" ? "?" : p ? `? ${g}` : g;
	} else if (a && !d || t == null && p) return g = `? ${g}`, f && !m ? g += ns(g, n.indent, l(f)) : h && i && i(), g;
	m && (f = null), p ? (f && (g += ns(g, n.indent, l(f))), g = `? ${g}\n${s}:`) : (g = `${g}:`, f && (g += ns(g, n.indent, l(f))));
	let _, v, y;
	H(t) ? (_ = !!t.spaceBefore, v = t.commentBefore, y = t.comment) : (_ = !1, v = null, y = null, t && typeof t == "object" && (t = o.createNode(t))), n.implicitKey = !1, !p && !f && B(t) && (n.indentAtStart = g.length + 1), h = !1, !u && c.length >= 2 && !n.inFlow && !p && Do(t) && !t.flow && !t.tag && !t.anchor && (n.indent = n.indent.substring(2));
	let b = !1, x = xs(t, n, () => b = !0, () => h = !0), S = " ";
	if (f || _ || v) {
		if (S = _ ? "\n" : "", v) {
			let e = l(v);
			S += `\n${q(e, n.indent)}`;
		}
		x === "" && !n.inFlow ? S === "\n" && y && (S = "\n\n") : S += `\n${n.indent}`;
	} else if (!p && V(t)) {
		let e = x[0], r = x.indexOf("\n"), i = r !== -1, a = n.inFlow ?? t.flow ?? t.items.length === 0;
		if (i || !a) {
			let t = !1;
			if (i && (e === "&" || e === "!")) {
				let n = x.indexOf(" ");
				e === "&" && n !== -1 && n < r && x[n + 1] === "!" && (n = x.indexOf(" ", n + 1)), (n === -1 || r < n) && (t = !0);
			}
			t || (S = `\n${n.indent}`);
		}
	} else (x === "" || x[0] === "\n") && (S = "");
	return g += S + x, n.inFlow ? b && r && r() : y && !b ? g += ns(g, n.indent, l(y)) : h && i && i(), g;
}
function Cs(e, t) {
	(e === "debug" || e === "warn") && console.warn(t);
}
var ws = "<<", J = {
	identify: (e) => e === ws || typeof e == "symbol" && e.description === ws,
	default: "key",
	tag: "tag:yaml.org,2002:merge",
	test: /^<<$/,
	resolve: () => Object.assign(new K(Symbol(ws)), { addToJSMap: Es }),
	stringify: () => ws
}, Ts = (e, t) => (J.identify(t) || B(t) && (!t.type || t.type === K.PLAIN) && J.identify(t.value)) && e?.doc.schema.tags.some((e) => e.tag === J.tag && e.default);
function Es(e, t, n) {
	let r = Os(e, n);
	if (Do(r)) for (let n of r.items) Ds(e, t, n);
	else if (Array.isArray(r)) for (let n of r) Ds(e, t, n);
	else Ds(e, t, r);
}
function Ds(e, t, n) {
	let r = Os(e, n);
	if (!Eo(r)) throw Error("Merge sources must be maps or map aliases");
	let i = r.toJSON(null, e, Map);
	for (let [e, n] of i) t instanceof Map ? t.has(e) || t.set(e, n) : t instanceof Set ? t.add(e) : Object.prototype.hasOwnProperty.call(t, e) || Object.defineProperty(t, e, {
		value: n,
		writable: !0,
		enumerable: !0,
		configurable: !0
	});
	return t;
}
function Os(e, t) {
	return e && wo(t) ? t.resolve(e.doc, e) : t;
}
function ks(e, t, { key: n, value: r }) {
	if (H(n) && n.addToJSMap) n.addToJSMap(e, t, r);
	else if (Ts(e, n)) Es(e, t, r);
	else {
		let i = G(n, "", e);
		if (t instanceof Map) t.set(i, G(r, i, e));
		else if (t instanceof Set) t.add(i);
		else {
			let a = As(n, i, e), o = G(r, a, e);
			a in t ? Object.defineProperty(t, a, {
				value: o,
				writable: !0,
				enumerable: !0,
				configurable: !0
			}) : t[a] = o;
		}
	}
	return t;
}
function As(e, t, n) {
	if (t === null) return "";
	if (typeof t != "object") return String(t);
	if (H(e) && n?.doc) {
		let t = vs(n.doc, {});
		t.anchors = /* @__PURE__ */ new Set();
		for (let e of n.anchors.keys()) t.anchors.add(e.anchor);
		t.inFlow = !0, t.inStringifyKey = !0;
		let r = e.toString(t);
		if (!n.mapKeyWarned) {
			let e = JSON.stringify(r);
			e.length > 40 && (e = e.substring(0, 36) + "...\""), Cs(n.doc.options.logLevel, `Keys with collection values will be stringified due to JS Object restrictions: ${e}. Set mapAsMap: true to use object keys.`), n.mapKeyWarned = !0;
		}
		return r;
	}
	return JSON.stringify(t);
}
function js(e, t, n) {
	return new Y(Zo(e, void 0, n), Zo(t, void 0, n));
}
var Y = class e {
	constructor(e, t = null) {
		Object.defineProperty(this, R, { value: So }), this.key = e, this.value = t;
	}
	clone(t) {
		let { key: n, value: r } = this;
		return H(n) && (n = n.clone(t)), H(r) && (r = r.clone(t)), new e(n, r);
	}
	toJSON(e, t) {
		return ks(t, t?.mapAsMap ? /* @__PURE__ */ new Map() : {}, this);
	}
	toString(e, t, n) {
		return e?.doc ? Ss(this, e, t, n) : JSON.stringify(this);
	}
};
function Ms(e, t, n) {
	return (t.inFlow ?? e.flow ? Ps : Ns)(e, t, n);
}
function Ns({ comment: e, items: t }, n, { blockItemPrefix: r, flowChars: i, itemIndent: a, onChompKeep: o, onComment: s }) {
	let { indent: c, options: { commentString: l } } = n, u = Object.assign({}, n, {
		indent: a,
		type: null
	}), d = !1, f = [];
	for (let e = 0; e < t.length; ++e) {
		let i = t[e], o = null;
		if (H(i)) !d && i.spaceBefore && f.push(""), Fs(n, f, i.commentBefore, d), i.comment && (o = i.comment);
		else if (z(i)) {
			let e = H(i.key) ? i.key : null;
			e && (!d && e.spaceBefore && f.push(""), Fs(n, f, e.commentBefore, d));
		}
		d = !1;
		let s = xs(i, u, () => o = null, () => d = !0);
		o && (s += ns(s, a, l(o))), d && o && (d = !1), f.push(r + s);
	}
	let p;
	if (f.length === 0) p = i.start + i.end;
	else {
		p = f[0];
		for (let e = 1; e < f.length; ++e) {
			let t = f[e];
			p += t ? `\n${c}${t}` : "\n";
		}
	}
	return e ? (p += "\n" + q(l(e), c), s && s()) : d && o && o(), p;
}
function Ps({ items: e }, t, { flowChars: n, itemIndent: r }) {
	let { indent: i, indentStep: a, flowCollectionPadding: o, options: { commentString: s } } = t;
	r += a;
	let c = Object.assign({}, t, {
		indent: r,
		inFlow: !0,
		type: null
	}), l = !1, u = 0, d = [];
	for (let n = 0; n < e.length; ++n) {
		let i = e[n], a = null;
		if (H(i)) i.spaceBefore && d.push(""), Fs(t, d, i.commentBefore, !1), i.comment && (a = i.comment);
		else if (z(i)) {
			let e = H(i.key) ? i.key : null;
			e && (e.spaceBefore && d.push(""), Fs(t, d, e.commentBefore, !1), e.comment && (l = !0));
			let n = H(i.value) ? i.value : null;
			n ? (n.comment && (a = n.comment), n.commentBefore && (l = !0)) : i.value == null && e?.comment && (a = e.comment);
		}
		a && (l = !0);
		let o = xs(i, c, () => a = null);
		l ||= d.length > u || o.includes("\n"), n < e.length - 1 ? o += "," : t.options.trailingComma && (t.options.lineWidth > 0 && (l ||= d.reduce((e, t) => e + t.length + 2, 2) + (o.length + 2) > t.options.lineWidth), l && (o += ",")), a && (o += ns(o, r, s(a))), d.push(o), u = d.length;
	}
	let { start: f, end: p } = n;
	if (d.length === 0) return f + p;
	if (!l) {
		let e = d.reduce((e, t) => e + t.length + 2, 2);
		l = t.options.lineWidth > 0 && e > t.options.lineWidth;
	}
	if (l) {
		let e = f;
		for (let t of d) e += t ? `\n${a}${i}${t}` : "\n";
		return `${e}\n${i}${p}`;
	} else return `${f}${o}${d.join(" ")}${o}${p}`;
}
function Fs({ indent: e, options: { commentString: t } }, n, r, i) {
	if (r && i && (r = r.replace(/^\n+/, "")), r) {
		let i = q(t(r), e);
		n.push(i.trimStart());
	}
}
function Is(e, t) {
	let n = B(t) ? t.value : t;
	for (let r of e) if (z(r) && (r.key === t || r.key === n || B(r.key) && r.key.value === n)) return r;
}
var X = class extends es {
	static get tagName() {
		return "tag:yaml.org,2002:map";
	}
	constructor(e) {
		super(xo, e), this.items = [];
	}
	static from(e, t, n) {
		let { keepUndefined: r, replacer: i } = n, a = new this(e), o = (e, o) => {
			if (typeof i == "function") o = i.call(t, e, o);
			else if (Array.isArray(i) && !i.includes(e)) return;
			(o !== void 0 || r) && a.items.push(js(e, o, n));
		};
		if (t instanceof Map) for (let [e, n] of t) o(e, n);
		else if (t && typeof t == "object") for (let e of Object.keys(t)) o(e, t[e]);
		return typeof e.sortMapEntries == "function" && a.items.sort(e.sortMapEntries), a;
	}
	add(e, t) {
		let n;
		n = z(e) ? e : !e || typeof e != "object" || !("key" in e) ? new Y(e, e?.value) : new Y(e.key, e.value);
		let r = Is(this.items, n.key), i = this.schema?.sortMapEntries;
		if (r) {
			if (!t) throw Error(`Key ${n.key} already set`);
			B(r.value) && Jo(n.value) ? r.value.value = n.value : r.value = n.value;
		} else if (i) {
			let e = this.items.findIndex((e) => i(n, e) < 0);
			e === -1 ? this.items.push(n) : this.items.splice(e, 0, n);
		} else this.items.push(n);
	}
	delete(e) {
		let t = Is(this.items, e);
		return t ? this.items.splice(this.items.indexOf(t), 1).length > 0 : !1;
	}
	get(e, t) {
		let n = Is(this.items, e)?.value;
		return (!t && B(n) ? n.value : n) ?? void 0;
	}
	has(e) {
		return !!Is(this.items, e);
	}
	set(e, t) {
		this.add(new Y(e, t), !0);
	}
	toJSON(e, t, n) {
		let r = n ? new n() : t?.mapAsMap ? /* @__PURE__ */ new Map() : {};
		t?.onCreate && t.onCreate(r);
		for (let e of this.items) ks(t, r, e);
		return r;
	}
	toString(e, t, n) {
		if (!e) return JSON.stringify(this);
		for (let e of this.items) if (!z(e)) throw Error(`Map items must all be pairs; found ${JSON.stringify(e)} instead`);
		return !e.allNullValues && this.hasAllNullValues(!1) && (e = Object.assign({}, e, { allNullValues: !0 })), Ms(this, e, {
			blockItemPrefix: "",
			flowChars: {
				start: "{",
				end: "}"
			},
			itemIndent: e.indent || "",
			onChompKeep: n,
			onComment: t
		});
	}
}, Ls = {
	collection: "map",
	default: !0,
	nodeClass: X,
	tag: "tag:yaml.org,2002:map",
	resolve(e, t) {
		return Eo(e) || t("Expected a mapping for this tag"), e;
	},
	createNode: (e, t, n) => X.from(e, t, n)
}, Rs = class extends es {
	static get tagName() {
		return "tag:yaml.org,2002:seq";
	}
	constructor(e) {
		super(Co, e), this.items = [];
	}
	add(e) {
		this.items.push(e);
	}
	delete(e) {
		let t = zs(e);
		return typeof t == "number" ? this.items.splice(t, 1).length > 0 : !1;
	}
	get(e, t) {
		let n = zs(e);
		if (typeof n != "number") return;
		let r = this.items[n];
		return !t && B(r) ? r.value : r;
	}
	has(e) {
		let t = zs(e);
		return typeof t == "number" && t < this.items.length;
	}
	set(e, t) {
		let n = zs(e);
		if (typeof n != "number") throw Error(`Expected a valid index, not ${e}.`);
		let r = this.items[n];
		B(r) && Jo(t) ? r.value = t : this.items[n] = t;
	}
	toJSON(e, t) {
		let n = [];
		t?.onCreate && t.onCreate(n);
		let r = 0;
		for (let e of this.items) n.push(G(e, String(r++), t));
		return n;
	}
	toString(e, t, n) {
		return e ? Ms(this, e, {
			blockItemPrefix: "- ",
			flowChars: {
				start: "[",
				end: "]"
			},
			itemIndent: (e.indent || "") + "  ",
			onChompKeep: n,
			onComment: t
		}) : JSON.stringify(this);
	}
	static from(e, t, n) {
		let { replacer: r } = n, i = new this(e);
		if (t && Symbol.iterator in Object(t)) {
			let e = 0;
			for (let a of t) {
				if (typeof r == "function") {
					let n = t instanceof Set ? a : String(e++);
					a = r.call(t, n, a);
				}
				i.items.push(Zo(a, void 0, n));
			}
		}
		return i;
	}
};
function zs(e) {
	let t = B(e) ? e.value : e;
	return t && typeof t == "string" && (t = Number(t)), typeof t == "number" && Number.isInteger(t) && t >= 0 ? t : null;
}
var Bs = {
	collection: "seq",
	default: !0,
	nodeClass: Rs,
	tag: "tag:yaml.org,2002:seq",
	resolve(e, t) {
		return Do(e) || t("Expected a sequence for this tag"), e;
	},
	createNode: (e, t, n) => Rs.from(e, t, n)
}, Vs = {
	identify: (e) => typeof e == "string",
	default: !0,
	tag: "tag:yaml.org,2002:str",
	resolve: (e) => e,
	stringify(e, t, n, r) {
		return t = Object.assign({ actualString: !0 }, t), _s(e, t, n, r);
	}
}, Hs = {
	identify: (e) => e == null,
	createNode: () => new K(null),
	default: !0,
	tag: "tag:yaml.org,2002:null",
	test: /^(?:~|[Nn]ull|NULL)?$/,
	resolve: () => new K(null),
	stringify: ({ source: e }, t) => typeof e == "string" && Hs.test.test(e) ? e : t.options.nullStr
}, Us = {
	identify: (e) => typeof e == "boolean",
	default: !0,
	tag: "tag:yaml.org,2002:bool",
	test: /^(?:[Tt]rue|TRUE|[Ff]alse|FALSE)$/,
	resolve: (e) => new K(e[0] === "t" || e[0] === "T"),
	stringify({ source: e, value: t }, n) {
		return e && Us.test.test(e) && t === (e[0] === "t" || e[0] === "T") ? e : t ? n.options.trueStr : n.options.falseStr;
	}
};
function Z({ format: e, minFractionDigits: t, tag: n, value: r }) {
	if (typeof r == "bigint") return String(r);
	let i = typeof r == "number" ? r : Number(r);
	if (!isFinite(i)) return isNaN(i) ? ".nan" : i < 0 ? "-.inf" : ".inf";
	let a = Object.is(r, -0) ? "-0" : JSON.stringify(r);
	if (!e && t && (!n || n === "tag:yaml.org,2002:float") && /^-?\d/.test(a) && !a.includes("e")) {
		let e = a.indexOf(".");
		e < 0 && (e = a.length, a += ".");
		let n = t - (a.length - e - 1);
		for (; n-- > 0;) a += "0";
	}
	return a;
}
var Ws = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	test: /^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$/,
	resolve: (e) => e.slice(-3).toLowerCase() === "nan" ? NaN : e[0] === "-" ? -Infinity : Infinity,
	stringify: Z
}, Gs = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	format: "EXP",
	test: /^[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)[eE][-+]?[0-9]+$/,
	resolve: (e) => parseFloat(e),
	stringify(e) {
		let t = Number(e.value);
		return isFinite(t) ? t.toExponential() : Z(e);
	}
}, Ks = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	test: /^[-+]?(?:\.[0-9]+|[0-9]+\.[0-9]*)$/,
	resolve(e) {
		let t = new K(parseFloat(e)), n = e.indexOf(".");
		return n !== -1 && e[e.length - 1] === "0" && (t.minFractionDigits = e.length - n - 1), t;
	},
	stringify: Z
}, qs = (e) => typeof e == "bigint" || Number.isInteger(e), Js = (e, t, n, { intAsBigInt: r }) => r ? BigInt(e) : parseInt(e.substring(t), n);
function Ys(e, t, n) {
	let { value: r } = e;
	return qs(r) && r >= 0 ? n + r.toString(t) : Z(e);
}
var Xs = {
	identify: (e) => qs(e) && e >= 0,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "OCT",
	test: /^0o[0-7]+$/,
	resolve: (e, t, n) => Js(e, 2, 8, n),
	stringify: (e) => Ys(e, 8, "0o")
}, Zs = {
	identify: qs,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	test: /^[-+]?[0-9]+$/,
	resolve: (e, t, n) => Js(e, 0, 10, n),
	stringify: Z
}, Qs = {
	identify: (e) => qs(e) && e >= 0,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "HEX",
	test: /^0x[0-9a-fA-F]+$/,
	resolve: (e, t, n) => Js(e, 2, 16, n),
	stringify: (e) => Ys(e, 16, "0x")
}, $s = [
	Ls,
	Bs,
	Vs,
	Hs,
	Us,
	Xs,
	Zs,
	Qs,
	Ws,
	Gs,
	Ks
];
function ec(e) {
	return typeof e == "bigint" || Number.isInteger(e);
}
var tc = ({ value: e }) => JSON.stringify(e), nc = [
	{
		identify: (e) => typeof e == "string",
		default: !0,
		tag: "tag:yaml.org,2002:str",
		resolve: (e) => e,
		stringify: tc
	},
	{
		identify: (e) => e == null,
		createNode: () => new K(null),
		default: !0,
		tag: "tag:yaml.org,2002:null",
		test: /^null$/,
		resolve: () => null,
		stringify: tc
	},
	{
		identify: (e) => typeof e == "boolean",
		default: !0,
		tag: "tag:yaml.org,2002:bool",
		test: /^true$|^false$/,
		resolve: (e) => e === "true",
		stringify: tc
	},
	{
		identify: ec,
		default: !0,
		tag: "tag:yaml.org,2002:int",
		test: /^-?(?:0|[1-9][0-9]*)$/,
		resolve: (e, t, { intAsBigInt: n }) => n ? BigInt(e) : parseInt(e, 10),
		stringify: ({ value: e }) => ec(e) ? e.toString() : JSON.stringify(e)
	},
	{
		identify: (e) => typeof e == "number",
		default: !0,
		tag: "tag:yaml.org,2002:float",
		test: /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]*)?(?:[eE][-+]?[0-9]+)?$/,
		resolve: (e) => parseFloat(e),
		stringify: tc
	}
], rc = [Ls, Bs].concat(nc, {
	default: !0,
	tag: "",
	test: /^/,
	resolve(e, t) {
		return t(`Unresolved plain scalar ${JSON.stringify(e)}`), e;
	}
}), ic = {
	identify: (e) => e instanceof Uint8Array,
	default: !1,
	tag: "tag:yaml.org,2002:binary",
	resolve(e, t) {
		if (typeof atob == "function") {
			let t = atob(e.replace(/[\n\r]/g, "")), n = new Uint8Array(t.length);
			for (let e = 0; e < t.length; ++e) n[e] = t.charCodeAt(e);
			return n;
		} else return t("This environment does not support reading binary tags; either Buffer or atob is required"), e;
	},
	stringify({ comment: e, type: t, value: n }, r, i, a) {
		if (!n) return "";
		let o = n, s;
		if (typeof btoa == "function") {
			let e = "";
			for (let t = 0; t < o.length; ++t) e += String.fromCharCode(o[t]);
			s = btoa(e);
		} else throw Error("This environment does not support writing binary tags; either Buffer or btoa is required");
		if (t ??= K.BLOCK_LITERAL, t !== K.QUOTE_DOUBLE) {
			let e = Math.max(r.options.lineWidth - r.indent.length, r.options.minContentWidth), n = Math.ceil(s.length / e), i = Array(n);
			for (let t = 0, r = 0; t < n; ++t, r += e) i[t] = s.substr(r, e);
			s = i.join(t === K.BLOCK_LITERAL ? "\n" : " ");
		}
		return _s({
			comment: e,
			type: t,
			value: s
		}, r, i, a);
	}
};
function ac(e, t) {
	if (Do(e)) for (let n = 0; n < e.items.length; ++n) {
		let r = e.items[n];
		if (!z(r)) {
			if (Eo(r)) {
				r.items.length > 1 && t("Each pair must have its own sequence indicator");
				let e = r.items[0] || new Y(new K(null));
				if (r.commentBefore && (e.key.commentBefore = e.key.commentBefore ? `${r.commentBefore}\n${e.key.commentBefore}` : r.commentBefore), r.comment) {
					let t = e.value ?? e.key;
					t.comment = t.comment ? `${r.comment}\n${t.comment}` : r.comment;
				}
				r = e;
			}
			e.items[n] = z(r) ? r : new Y(r);
		}
	}
	else t("Expected a sequence for this tag");
	return e;
}
function oc(e, t, n) {
	let { replacer: r } = n, i = new Rs(e);
	i.tag = "tag:yaml.org,2002:pairs";
	let a = 0;
	if (t && Symbol.iterator in Object(t)) for (let e of t) {
		typeof r == "function" && (e = r.call(t, String(a++), e));
		let o, s;
		if (Array.isArray(e)) if (e.length === 2) o = e[0], s = e[1];
		else throw TypeError(`Expected [key, value] tuple: ${e}`);
		else if (e && e instanceof Object) {
			let t = Object.keys(e);
			if (t.length === 1) o = t[0], s = e[o];
			else throw TypeError(`Expected tuple with one key, not ${t.length} keys`);
		} else o = e;
		i.items.push(js(o, s, n));
	}
	return i;
}
var sc = {
	collection: "seq",
	default: !1,
	tag: "tag:yaml.org,2002:pairs",
	resolve: ac,
	createNode: oc
}, cc = class e extends Rs {
	constructor() {
		super(), this.add = X.prototype.add.bind(this), this.delete = X.prototype.delete.bind(this), this.get = X.prototype.get.bind(this), this.has = X.prototype.has.bind(this), this.set = X.prototype.set.bind(this), this.tag = e.tag;
	}
	toJSON(e, t) {
		if (!t) return super.toJSON(e);
		let n = /* @__PURE__ */ new Map();
		t?.onCreate && t.onCreate(n);
		for (let e of this.items) {
			let r, i;
			if (z(e) ? (r = G(e.key, "", t), i = G(e.value, r, t)) : r = G(e, "", t), n.has(r)) throw Error("Ordered maps must not include duplicate keys");
			n.set(r, i);
		}
		return n;
	}
	static from(e, t, n) {
		let r = oc(e, t, n), i = new this();
		return i.items = r.items, i;
	}
};
cc.tag = "tag:yaml.org,2002:omap";
var lc = {
	collection: "seq",
	identify: (e) => e instanceof Map,
	nodeClass: cc,
	default: !1,
	tag: "tag:yaml.org,2002:omap",
	resolve(e, t) {
		let n = ac(e, t), r = [];
		for (let { key: e } of n.items) B(e) && (r.includes(e.value) ? t(`Ordered maps must not include duplicate keys: ${e.value}`) : r.push(e.value));
		return Object.assign(new cc(), n);
	},
	createNode: (e, t, n) => cc.from(e, t, n)
};
function uc({ value: e, source: t }, n) {
	return t && (e ? dc : fc).test.test(t) ? t : e ? n.options.trueStr : n.options.falseStr;
}
var dc = {
	identify: (e) => e === !0,
	default: !0,
	tag: "tag:yaml.org,2002:bool",
	test: /^(?:Y|y|[Yy]es|YES|[Tt]rue|TRUE|[Oo]n|ON)$/,
	resolve: () => new K(!0),
	stringify: uc
}, fc = {
	identify: (e) => e === !1,
	default: !0,
	tag: "tag:yaml.org,2002:bool",
	test: /^(?:N|n|[Nn]o|NO|[Ff]alse|FALSE|[Oo]ff|OFF)$/,
	resolve: () => new K(!1),
	stringify: uc
}, pc = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	test: /^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$/,
	resolve: (e) => e.slice(-3).toLowerCase() === "nan" ? NaN : e[0] === "-" ? -Infinity : Infinity,
	stringify: Z
}, mc = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	format: "EXP",
	test: /^[-+]?(?:[0-9][0-9_]*)?(?:\.[0-9_]*)?[eE][-+]?[0-9]+$/,
	resolve: (e) => parseFloat(e.replace(/_/g, "")),
	stringify(e) {
		let t = Number(e.value);
		return isFinite(t) ? t.toExponential() : Z(e);
	}
}, hc = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	test: /^[-+]?(?:[0-9][0-9_]*)?\.[0-9_]*$/,
	resolve(e) {
		let t = new K(parseFloat(e.replace(/_/g, ""))), n = e.indexOf(".");
		if (n !== -1) {
			let r = e.substring(n + 1).replace(/_/g, "");
			r[r.length - 1] === "0" && (t.minFractionDigits = r.length);
		}
		return t;
	},
	stringify: Z
}, gc = (e) => typeof e == "bigint" || Number.isInteger(e);
function _c(e, t, n, { intAsBigInt: r }) {
	let i = e[0];
	if ((i === "-" || i === "+") && (t += 1), e = e.substring(t).replace(/_/g, ""), r) {
		switch (n) {
			case 2:
				e = `0b${e}`;
				break;
			case 8:
				e = `0o${e}`;
				break;
			case 16:
				e = `0x${e}`;
				break;
		}
		let t = BigInt(e);
		return i === "-" ? BigInt(-1) * t : t;
	}
	let a = parseInt(e, n);
	return i === "-" ? -1 * a : a;
}
function vc(e, t, n) {
	let { value: r } = e;
	if (gc(r)) {
		let e = r.toString(t);
		return r < 0 ? "-" + n + e.substr(1) : n + e;
	}
	return Z(e);
}
var yc = {
	identify: gc,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "BIN",
	test: /^[-+]?0b[0-1_]+$/,
	resolve: (e, t, n) => _c(e, 2, 2, n),
	stringify: (e) => vc(e, 2, "0b")
}, bc = {
	identify: gc,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "OCT",
	test: /^[-+]?0[0-7_]+$/,
	resolve: (e, t, n) => _c(e, 1, 8, n),
	stringify: (e) => vc(e, 8, "0")
}, xc = {
	identify: gc,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	test: /^[-+]?[0-9][0-9_]*$/,
	resolve: (e, t, n) => _c(e, 0, 10, n),
	stringify: Z
}, Sc = {
	identify: gc,
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "HEX",
	test: /^[-+]?0x[0-9a-fA-F_]+$/,
	resolve: (e, t, n) => _c(e, 2, 16, n),
	stringify: (e) => vc(e, 16, "0x")
}, Cc = class e extends X {
	constructor(t) {
		super(t), this.tag = e.tag;
	}
	add(e) {
		let t;
		t = z(e) ? e : e && typeof e == "object" && "key" in e && "value" in e && e.value === null ? new Y(e.key, null) : new Y(e, null), Is(this.items, t.key) || this.items.push(t);
	}
	get(e, t) {
		let n = Is(this.items, e);
		return !t && z(n) ? B(n.key) ? n.key.value : n.key : n;
	}
	set(e, t) {
		if (typeof t != "boolean") throw Error(`Expected boolean value for set(key, value) in a YAML set, not ${typeof t}`);
		let n = Is(this.items, e);
		n && !t ? this.items.splice(this.items.indexOf(n), 1) : !n && t && this.items.push(new Y(e));
	}
	toJSON(e, t) {
		return super.toJSON(e, t, Set);
	}
	toString(e, t, n) {
		if (!e) return JSON.stringify(this);
		if (this.hasAllNullValues(!0)) return super.toString(Object.assign({}, e, { allNullValues: !0 }), t, n);
		throw Error("Set items must all have null values");
	}
	static from(e, t, n) {
		let { replacer: r } = n, i = new this(e);
		if (t && Symbol.iterator in Object(t)) for (let e of t) typeof r == "function" && (e = r.call(t, e, e)), i.items.push(js(e, null, n));
		return i;
	}
};
Cc.tag = "tag:yaml.org,2002:set";
var wc = {
	collection: "map",
	identify: (e) => e instanceof Set,
	nodeClass: Cc,
	default: !1,
	tag: "tag:yaml.org,2002:set",
	createNode: (e, t, n) => Cc.from(e, t, n),
	resolve(e, t) {
		if (Eo(e)) {
			if (e.hasAllNullValues(!0)) return Object.assign(new Cc(), e);
			t("Set items must all have null values");
		} else t("Expected a mapping for this tag");
		return e;
	}
};
function Tc(e, t) {
	let n = e[0], r = n === "-" || n === "+" ? e.substring(1) : e, i = (e) => t ? BigInt(e) : Number(e), a = r.replace(/_/g, "").split(":").reduce((e, t) => e * i(60) + i(t), i(0));
	return n === "-" ? i(-1) * a : a;
}
function Ec(e) {
	let { value: t } = e, n = (e) => e;
	if (typeof t == "bigint") n = (e) => BigInt(e);
	else if (isNaN(t) || !isFinite(t)) return Z(e);
	let r = "";
	t < 0 && (r = "-", t *= n(-1));
	let i = n(60), a = [t % i];
	return t < 60 ? a.unshift(0) : (t = (t - a[0]) / i, a.unshift(t % i), t >= 60 && (t = (t - a[0]) / i, a.unshift(t))), r + a.map((e) => String(e).padStart(2, "0")).join(":").replace(/000000\d*$/, "");
}
var Dc = {
	identify: (e) => typeof e == "bigint" || Number.isInteger(e),
	default: !0,
	tag: "tag:yaml.org,2002:int",
	format: "TIME",
	test: /^[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+$/,
	resolve: (e, t, { intAsBigInt: n }) => Tc(e, n),
	stringify: Ec
}, Oc = {
	identify: (e) => typeof e == "number",
	default: !0,
	tag: "tag:yaml.org,2002:float",
	format: "TIME",
	test: /^[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*$/,
	resolve: (e) => Tc(e, !1),
	stringify: Ec
}, kc = {
	identify: (e) => e instanceof Date,
	default: !0,
	tag: "tag:yaml.org,2002:timestamp",
	test: RegExp("^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})(?:(?:t|T|[ \\t]+)([0-9]{1,2}):([0-9]{1,2}):([0-9]{1,2}(\\.[0-9]+)?)(?:[ \\t]*(Z|[-+][012]?[0-9](?::[0-9]{2})?))?)?$"),
	resolve(e) {
		let t = e.match(kc.test);
		if (!t) throw Error("!!timestamp expects a date, starting with yyyy-mm-dd");
		let [, n, r, i, a, o, s] = t.map(Number), c = t[7] ? Number((t[7] + "00").substr(1, 3)) : 0, l = Date.UTC(n, r - 1, i, a || 0, o || 0, s || 0, c), u = t[8];
		if (u && u !== "Z") {
			let e = Tc(u, !1);
			Math.abs(e) < 30 && (e *= 60), l -= 6e4 * e;
		}
		return new Date(l);
	},
	stringify: ({ value: e }) => e?.toISOString().replace(/(T00:00:00)?\.000Z$/, "") ?? ""
}, Ac = [
	Ls,
	Bs,
	Vs,
	Hs,
	dc,
	fc,
	yc,
	bc,
	xc,
	Sc,
	pc,
	mc,
	hc,
	ic,
	J,
	lc,
	sc,
	wc,
	Dc,
	Oc,
	kc
], jc = new Map([
	["core", $s],
	["failsafe", [
		Ls,
		Bs,
		Vs
	]],
	["json", rc],
	["yaml11", Ac],
	["yaml-1.1", Ac]
]), Mc = {
	binary: ic,
	bool: Us,
	float: Ks,
	floatExp: Gs,
	floatNaN: Ws,
	floatTime: Oc,
	int: Zs,
	intHex: Qs,
	intOct: Xs,
	intTime: Dc,
	map: Ls,
	merge: J,
	null: Hs,
	omap: lc,
	pairs: sc,
	seq: Bs,
	set: wc,
	timestamp: kc
}, Nc = {
	"tag:yaml.org,2002:binary": ic,
	"tag:yaml.org,2002:merge": J,
	"tag:yaml.org,2002:omap": lc,
	"tag:yaml.org,2002:pairs": sc,
	"tag:yaml.org,2002:set": wc,
	"tag:yaml.org,2002:timestamp": kc
};
function Pc(e, t, n) {
	let r = jc.get(t);
	if (r && !e) return n && !r.includes(J) ? r.concat(J) : r.slice();
	let i = r;
	if (!i) if (Array.isArray(e)) i = [];
	else {
		let e = Array.from(jc.keys()).filter((e) => e !== "yaml11").map((e) => JSON.stringify(e)).join(", ");
		throw Error(`Unknown schema "${t}"; use one of ${e} or define customTags array`);
	}
	if (Array.isArray(e)) for (let t of e) i = i.concat(t);
	else typeof e == "function" && (i = e(i.slice()));
	return n && (i = i.concat(J)), i.reduce((e, t) => {
		let n = typeof t == "string" ? Mc[t] : t;
		if (!n) {
			let e = JSON.stringify(t), n = Object.keys(Mc).map((e) => JSON.stringify(e)).join(", ");
			throw Error(`Unknown custom tag ${e}; use one of ${n}`);
		}
		return e.includes(n) || e.push(n), e;
	}, []);
}
var Fc = (e, t) => e.key < t.key ? -1 : +(e.key > t.key), Ic = class e {
	constructor({ compat: e, customTags: t, merge: n, resolveKnownTags: r, schema: i, sortMapEntries: a, toStringDefaults: o }) {
		this.compat = Array.isArray(e) ? Pc(e, "compat") : e ? Pc(null, e) : null, this.name = typeof i == "string" && i || "core", this.knownTags = r ? Nc : {}, this.tags = Pc(t, this.name, n), this.toStringOptions = o ?? null, Object.defineProperty(this, xo, { value: Ls }), Object.defineProperty(this, L, { value: Vs }), Object.defineProperty(this, Co, { value: Bs }), this.sortMapEntries = typeof a == "function" ? a : a === !0 ? Fc : null;
	}
	clone() {
		let t = Object.create(e.prototype, Object.getOwnPropertyDescriptors(this));
		return t.tags = this.tags.slice(), t;
	}
};
function Lc(e, t) {
	let n = [], r = t.directives === !0;
	if (t.directives !== !1 && e.directives) {
		let t = e.directives.toString(e);
		t ? (n.push(t), r = !0) : e.directives.docStart && (r = !0);
	}
	r && n.push("---");
	let i = vs(e, t), { commentString: a } = i.options;
	if (e.commentBefore) {
		n.length !== 1 && n.unshift("");
		let t = a(e.commentBefore);
		n.unshift(q(t, ""));
	}
	let o = !1, s = null;
	if (e.contents) {
		if (H(e.contents)) {
			if (e.contents.spaceBefore && r && n.push(""), e.contents.commentBefore) {
				let t = a(e.contents.commentBefore);
				n.push(q(t, ""));
			}
			i.forceBlockIndent = !!e.comment, s = e.contents.comment;
		}
		let t = s ? void 0 : () => o = !0, c = xs(e.contents, i, () => s = null, t);
		s && (c += ns(c, "", a(s))), (c[0] === "|" || c[0] === ">") && n[n.length - 1] === "---" ? n[n.length - 1] = `--- ${c}` : n.push(c);
	} else n.push(xs(e.contents, i));
	if (e.directives?.docEnd) if (e.comment) {
		let t = a(e.comment);
		t.includes("\n") ? (n.push("..."), n.push(q(t, ""))) : n.push(`... ${t}`);
	} else n.push("...");
	else {
		let t = e.comment;
		t && o && (t = t.replace(/^\n+/, "")), t && ((!o || s) && n[n.length - 1] !== "" && n.push(""), n.push(q(a(t), "")));
	}
	return n.join("\n") + "\n";
}
var Rc = class e {
	constructor(e, t, n) {
		this.commentBefore = null, this.comment = null, this.errors = [], this.warnings = [], Object.defineProperty(this, R, { value: bo });
		let r = null;
		typeof t == "function" || Array.isArray(t) ? r = t : n === void 0 && t && (n = t, t = void 0);
		let i = Object.assign({
			intAsBigInt: !1,
			keepSourceTokens: !1,
			logLevel: "warn",
			prettyErrors: !0,
			strict: !0,
			stringKeys: !1,
			uniqueKeys: !0,
			version: "1.2"
		}, n);
		this.options = i;
		let { version: a } = i;
		n?._directives ? (this.directives = n._directives.atDocument(), this.directives.yaml.explicit && (a = this.directives.yaml.version)) : this.directives = new zo({ version: a }), this.setSchema(a, n), this.contents = e === void 0 ? null : this.createNode(e, r, n);
	}
	clone() {
		let t = Object.create(e.prototype, { [R]: { value: bo } });
		return t.commentBefore = this.commentBefore, t.comment = this.comment, t.errors = this.errors.slice(), t.warnings = this.warnings.slice(), t.options = Object.assign({}, this.options), this.directives && (t.directives = this.directives.clone()), t.schema = this.schema.clone(), t.contents = H(this.contents) ? this.contents.clone(t.schema) : this.contents, this.range && (t.range = this.range.slice()), t;
	}
	add(e) {
		zc(this.contents) && this.contents.add(e);
	}
	addIn(e, t) {
		zc(this.contents) && this.contents.addIn(e, t);
	}
	createAlias(e, t) {
		if (!e.anchor) {
			let n = Vo(this);
			e.anchor = !t || n.has(t) ? Ho(t || "a", n) : t;
		}
		return new Ko(e.anchor);
	}
	createNode(e, t, n) {
		let r;
		if (typeof t == "function") e = t.call({ "": e }, "", e), r = t;
		else if (Array.isArray(t)) {
			let e = t.filter((e) => typeof e == "number" || e instanceof String || e instanceof Number).map(String);
			e.length > 0 && (t = t.concat(e)), r = t;
		} else n === void 0 && t && (n = t, t = void 0);
		let { aliasDuplicateObjects: i, anchorPrefix: a, flow: o, keepUndefined: s, onTagObj: c, tag: l } = n ?? {}, { onAnchor: u, setAnchors: d, sourceObjects: f } = Uo(this, a || "a"), p = {
			aliasDuplicateObjects: i ?? !0,
			keepUndefined: s ?? !1,
			onAnchor: u,
			onTagObj: c,
			replacer: r,
			schema: this.schema,
			sourceObjects: f
		}, m = Zo(e, l, p);
		return o && V(m) && (m.flow = !0), d(), m;
	}
	createPair(e, t, n = {}) {
		return new Y(this.createNode(e, null, n), this.createNode(t, null, n));
	}
	delete(e) {
		return zc(this.contents) ? this.contents.delete(e) : !1;
	}
	deleteIn(e) {
		return $o(e) ? this.contents == null ? !1 : (this.contents = null, !0) : zc(this.contents) ? this.contents.deleteIn(e) : !1;
	}
	get(e, t) {
		return V(this.contents) ? this.contents.get(e, t) : void 0;
	}
	getIn(e, t) {
		return $o(e) ? !t && B(this.contents) ? this.contents.value : this.contents : V(this.contents) ? this.contents.getIn(e, t) : void 0;
	}
	has(e) {
		return V(this.contents) ? this.contents.has(e) : !1;
	}
	hasIn(e) {
		return $o(e) ? this.contents !== void 0 : V(this.contents) ? this.contents.hasIn(e) : !1;
	}
	set(e, t) {
		this.contents == null ? this.contents = Qo(this.schema, [e], t) : zc(this.contents) && this.contents.set(e, t);
	}
	setIn(e, t) {
		$o(e) ? this.contents = t : this.contents == null ? this.contents = Qo(this.schema, Array.from(e), t) : zc(this.contents) && this.contents.setIn(e, t);
	}
	setSchema(e, t = {}) {
		typeof e == "number" && (e = String(e));
		let n;
		switch (e) {
			case "1.1":
				this.directives ? this.directives.yaml.version = "1.1" : this.directives = new zo({ version: "1.1" }), n = {
					resolveKnownTags: !1,
					schema: "yaml-1.1"
				};
				break;
			case "1.2":
			case "next":
				this.directives ? this.directives.yaml.version = e : this.directives = new zo({ version: e }), n = {
					resolveKnownTags: !0,
					schema: "core"
				};
				break;
			case null:
				this.directives && delete this.directives, n = null;
				break;
			default: {
				let t = JSON.stringify(e);
				throw Error(`Expected '1.1', '1.2' or null as first argument, but found: ${t}`);
			}
		}
		if (t.schema instanceof Object) this.schema = t.schema;
		else if (n) this.schema = new Ic(Object.assign(n, t));
		else throw Error("With a null YAML version, the { schema: Schema } option is required");
	}
	toJS({ json: e, jsonArg: t, mapAsMap: n, maxAliasCount: r, onAnchor: i, reviver: a } = {}) {
		let o = {
			anchors: /* @__PURE__ */ new Map(),
			doc: this,
			keep: !e,
			mapAsMap: n === !0,
			mapKeyWarned: !1,
			maxAliasCount: typeof r == "number" ? r : 100
		}, s = G(this.contents, t ?? "", o);
		if (typeof i == "function") for (let { count: e, res: t } of o.anchors.values()) i(t, e);
		return typeof a == "function" ? Wo(a, { "": s }, "", s) : s;
	}
	toJSON(e, t) {
		return this.toJS({
			json: !0,
			jsonArg: e,
			mapAsMap: !1,
			onAnchor: t
		});
	}
	toString(e = {}) {
		if (this.errors.length > 0) throw Error("Document with errors cannot be stringified");
		if ("indent" in e && (!Number.isInteger(e.indent) || Number(e.indent) <= 0)) {
			let t = JSON.stringify(e.indent);
			throw Error(`"indent" option must be a positive integer, not ${t}`);
		}
		return Lc(this, e);
	}
};
function zc(e) {
	if (V(e)) return !0;
	throw Error("Expected a YAML collection as document contents");
}
var Bc = class extends Error {
	constructor(e, t, n, r) {
		super(), this.name = e, this.code = n, this.message = r, this.pos = t;
	}
}, Vc = class extends Bc {
	constructor(e, t, n) {
		super("YAMLParseError", e, t, n);
	}
}, Hc = class extends Bc {
	constructor(e, t, n) {
		super("YAMLWarning", e, t, n);
	}
}, Uc = (e, t) => (n) => {
	if (n.pos[0] === -1) return;
	n.linePos = n.pos.map((e) => t.linePos(e));
	let { line: r, col: i } = n.linePos[0];
	n.message += ` at line ${r}, column ${i}`;
	let a = i - 1, o = e.substring(t.lineStarts[r - 1], t.lineStarts[r]).replace(/[\n\r]+$/, "");
	if (a >= 60 && o.length > 80) {
		let e = Math.min(a - 39, o.length - 79);
		o = "…" + o.substring(e), a -= e - 1;
	}
	if (o.length > 80 && (o = o.substring(0, 79) + "…"), r > 1 && /^ *$/.test(o.substring(0, a))) {
		let n = e.substring(t.lineStarts[r - 2], t.lineStarts[r - 1]);
		n.length > 80 && (n = n.substring(0, 79) + "…\n"), o = n + o;
	}
	if (/[^ ]/.test(o)) {
		let e = 1, t = n.linePos[1];
		t?.line === r && t.col > i && (e = Math.max(1, Math.min(t.col - i, 80 - a)));
		let s = " ".repeat(a) + "^".repeat(e);
		n.message += `:\n\n${o}\n${s}\n`;
	}
};
function Wc(e, { flow: t, indicator: n, next: r, offset: i, onError: a, parentIndent: o, startOnNewline: s }) {
	let c = !1, l = s, u = s, d = "", f = "", p = !1, m = !1, h = null, g = null, _ = null, v = null, y = null, b = null, x = null;
	for (let i of e) switch (m &&= (i.type !== "space" && i.type !== "newline" && i.type !== "comma" && a(i.offset, "MISSING_CHAR", "Tags and anchors must be separated from the next token by white space"), !1), h &&= (l && i.type !== "comment" && i.type !== "newline" && a(h, "TAB_AS_INDENT", "Tabs are not allowed as indentation"), null), i.type) {
		case "space":
			!t && (n !== "doc-start" || r?.type !== "flow-collection") && i.source.includes("	") && (h = i), u = !0;
			break;
		case "comment": {
			u || a(i, "MISSING_CHAR", "Comments must be separated from other tokens by white space characters");
			let e = i.source.substring(1) || " ";
			d ? d += f + e : d = e, f = "", l = !1;
			break;
		}
		case "newline":
			l ? d ? d += i.source : (!b || n !== "seq-item-ind") && (c = !0) : f += i.source, l = !0, p = !0, (g || _) && (v = i), u = !0;
			break;
		case "anchor":
			g && a(i, "MULTIPLE_ANCHORS", "A node can have at most one anchor"), i.source.endsWith(":") && a(i.offset + i.source.length - 1, "BAD_ALIAS", "Anchor ending in : is ambiguous", !0), g = i, x ??= i.offset, l = !1, u = !1, m = !0;
			break;
		case "tag":
			_ && a(i, "MULTIPLE_TAGS", "A node can have at most one tag"), _ = i, x ??= i.offset, l = !1, u = !1, m = !0;
			break;
		case n:
			(g || _) && a(i, "BAD_PROP_ORDER", `Anchors and tags must be after the ${i.source} indicator`), b && a(i, "UNEXPECTED_TOKEN", `Unexpected ${i.source} in ${t ?? "collection"}`), b = i, l = n === "seq-item-ind" || n === "explicit-key-ind", u = !1;
			break;
		case "comma": if (t) {
			y && a(i, "UNEXPECTED_TOKEN", `Unexpected , in ${t}`), y = i, l = !1, u = !1;
			break;
		}
		default: a(i, "UNEXPECTED_TOKEN", `Unexpected ${i.type} token`), l = !1, u = !1;
	}
	let S = e[e.length - 1], ee = S ? S.offset + S.source.length : i;
	return m && r && r.type !== "space" && r.type !== "newline" && r.type !== "comma" && (r.type !== "scalar" || r.source !== "") && a(r.offset, "MISSING_CHAR", "Tags and anchors must be separated from the next token by white space"), h && (l && h.indent <= o || r?.type === "block-map" || r?.type === "block-seq") && a(h, "TAB_AS_INDENT", "Tabs are not allowed as indentation"), {
		comma: y,
		found: b,
		spaceBefore: c,
		comment: d,
		hasNewline: p,
		anchor: g,
		tag: _,
		newlineAfterProp: v,
		end: ee,
		start: x ?? ee
	};
}
function Gc(e) {
	if (!e) return null;
	switch (e.type) {
		case "alias":
		case "scalar":
		case "double-quoted-scalar":
		case "single-quoted-scalar":
			if (e.source.includes("\n")) return !0;
			if (e.end) {
				for (let t of e.end) if (t.type === "newline") return !0;
			}
			return !1;
		case "flow-collection":
			for (let t of e.items) {
				for (let e of t.start) if (e.type === "newline") return !0;
				if (t.sep) {
					for (let e of t.sep) if (e.type === "newline") return !0;
				}
				if (Gc(t.key) || Gc(t.value)) return !0;
			}
			return !1;
		default: return !0;
	}
}
function Kc(e, t, n) {
	if (t?.type === "flow-collection") {
		let r = t.end[0];
		r.indent === e && (r.source === "]" || r.source === "}") && Gc(t) && n(r, "BAD_INDENT", "Flow end indicator should be more indented than parent", !0);
	}
}
function qc(e, t, n) {
	let { uniqueKeys: r } = e.options;
	if (r === !1) return !1;
	let i = typeof r == "function" ? r : (e, t) => e === t || B(e) && B(t) && e.value === t.value;
	return t.some((e) => i(e.key, n));
}
var Jc = "All mapping items must start at the same column";
function Yc({ composeNode: e, composeEmptyNode: t }, n, r, i, a) {
	let o = new (a?.nodeClass ?? X)(n.schema);
	n.atRoot &&= !1;
	let s = r.offset, c = null;
	for (let a of r.items) {
		let { start: l, key: u, sep: d, value: f } = a, p = Wc(l, {
			indicator: "explicit-key-ind",
			next: u ?? d?.[0],
			offset: s,
			onError: i,
			parentIndent: r.indent,
			startOnNewline: !0
		}), m = !p.found;
		if (m) {
			if (u && (u.type === "block-seq" ? i(s, "BLOCK_AS_IMPLICIT_KEY", "A block sequence may not be used as an implicit map key") : "indent" in u && u.indent !== r.indent && i(s, "BAD_INDENT", Jc)), !p.anchor && !p.tag && !d) {
				c = p.end, p.comment && (o.comment ? o.comment += "\n" + p.comment : o.comment = p.comment);
				continue;
			}
			(p.newlineAfterProp || Gc(u)) && i(u ?? l[l.length - 1], "MULTILINE_IMPLICIT_KEY", "Implicit keys need to be on a single line");
		} else p.found?.indent !== r.indent && i(s, "BAD_INDENT", Jc);
		n.atKey = !0;
		let h = p.end, g = u ? e(n, u, p, i) : t(n, h, l, null, p, i);
		n.schema.compat && Kc(r.indent, u, i), n.atKey = !1, qc(n, o.items, g) && i(h, "DUPLICATE_KEY", "Map keys must be unique");
		let _ = Wc(d ?? [], {
			indicator: "map-value-ind",
			next: f,
			offset: g.range[2],
			onError: i,
			parentIndent: r.indent,
			startOnNewline: !u || u.type === "block-scalar"
		});
		if (s = _.end, _.found) {
			m && (f?.type === "block-map" && !_.hasNewline && i(s, "BLOCK_AS_IMPLICIT_KEY", "Nested mappings are not allowed in compact mappings"), n.options.strict && p.start < _.found.offset - 1024 && i(g.range, "KEY_OVER_1024_CHARS", "The : indicator must be at most 1024 chars after the start of an implicit block mapping key"));
			let c = f ? e(n, f, _, i) : t(n, s, d, null, _, i);
			n.schema.compat && Kc(r.indent, f, i), s = c.range[2];
			let l = new Y(g, c);
			n.options.keepSourceTokens && (l.srcToken = a), o.items.push(l);
		} else {
			m && i(g.range, "MISSING_CHAR", "Implicit map keys need to be followed by map values"), _.comment && (g.comment ? g.comment += "\n" + _.comment : g.comment = _.comment);
			let e = new Y(g);
			n.options.keepSourceTokens && (e.srcToken = a), o.items.push(e);
		}
	}
	return c && c < s && i(c, "IMPOSSIBLE", "Map comment with trailing content"), o.range = [
		r.offset,
		s,
		c ?? s
	], o;
}
function Xc({ composeNode: e, composeEmptyNode: t }, n, r, i, a) {
	let o = new (a?.nodeClass ?? Rs)(n.schema);
	n.atRoot &&= !1, n.atKey &&= !1;
	let s = r.offset, c = null;
	for (let { start: a, value: l } of r.items) {
		let u = Wc(a, {
			indicator: "seq-item-ind",
			next: l,
			offset: s,
			onError: i,
			parentIndent: r.indent,
			startOnNewline: !0
		});
		if (!u.found) if (u.anchor || u.tag || l) l?.type === "block-seq" ? i(u.end, "BAD_INDENT", "All sequence items must start at the same column") : i(s, "MISSING_CHAR", "Sequence item without - indicator");
		else {
			c = u.end, u.comment && (o.comment = u.comment);
			continue;
		}
		let d = l ? e(n, l, u, i) : t(n, u.end, a, null, u, i);
		n.schema.compat && Kc(r.indent, l, i), s = d.range[2], o.items.push(d);
	}
	return o.range = [
		r.offset,
		s,
		c ?? s
	], o;
}
function Zc(e, t, n, r) {
	let i = "";
	if (e) {
		let a = !1, o = "";
		for (let s of e) {
			let { source: e, type: c } = s;
			switch (c) {
				case "space":
					a = !0;
					break;
				case "comment": {
					n && !a && r(s, "MISSING_CHAR", "Comments must be separated from other tokens by white space characters");
					let t = e.substring(1) || " ";
					i ? i += o + t : i = t, o = "";
					break;
				}
				case "newline":
					i && (o += e), a = !0;
					break;
				default: r(s, "UNEXPECTED_TOKEN", `Unexpected ${c} at node end`);
			}
			t += e.length;
		}
	}
	return {
		comment: i,
		offset: t
	};
}
var Qc = "Block collections are not allowed within flow collections", $c = (e) => e && (e.type === "block-map" || e.type === "block-seq");
function el({ composeNode: e, composeEmptyNode: t }, n, r, i, a) {
	let o = r.start.source === "{", s = o ? "flow map" : "flow sequence", c = new (a?.nodeClass ?? (o ? X : Rs))(n.schema);
	c.flow = !0;
	let l = n.atRoot;
	l && (n.atRoot = !1), n.atKey &&= !1;
	let u = r.offset + r.start.source.length;
	for (let a = 0; a < r.items.length; ++a) {
		let l = r.items[a], { start: d, key: f, sep: p, value: m } = l, h = Wc(d, {
			flow: s,
			indicator: "explicit-key-ind",
			next: f ?? p?.[0],
			offset: u,
			onError: i,
			parentIndent: r.indent,
			startOnNewline: !1
		});
		if (!h.found) {
			if (!h.anchor && !h.tag && !p && !m) {
				a === 0 && h.comma ? i(h.comma, "UNEXPECTED_TOKEN", `Unexpected , in ${s}`) : a < r.items.length - 1 && i(h.start, "UNEXPECTED_TOKEN", `Unexpected empty item in ${s}`), h.comment && (c.comment ? c.comment += "\n" + h.comment : c.comment = h.comment), u = h.end;
				continue;
			}
			!o && n.options.strict && Gc(f) && i(f, "MULTILINE_IMPLICIT_KEY", "Implicit keys of flow sequence pairs need to be on a single line");
		}
		if (a === 0) h.comma && i(h.comma, "UNEXPECTED_TOKEN", `Unexpected , in ${s}`);
		else if (h.comma || i(h.start, "MISSING_CHAR", `Missing , between ${s} items`), h.comment) {
			let e = "";
			loop: for (let t of d) switch (t.type) {
				case "comma":
				case "space": break;
				case "comment":
					e = t.source.substring(1);
					break loop;
				default: break loop;
			}
			if (e) {
				let t = c.items[c.items.length - 1];
				z(t) && (t = t.value ?? t.key), t.comment ? t.comment += "\n" + e : t.comment = e, h.comment = h.comment.substring(e.length + 1);
			}
		}
		if (!o && !p && !h.found) {
			let r = m ? e(n, m, h, i) : t(n, h.end, p, null, h, i);
			c.items.push(r), u = r.range[2], $c(m) && i(r.range, "BLOCK_IN_FLOW", Qc);
		} else {
			n.atKey = !0;
			let a = h.end, g = f ? e(n, f, h, i) : t(n, a, d, null, h, i);
			$c(f) && i(g.range, "BLOCK_IN_FLOW", Qc), n.atKey = !1;
			let _ = Wc(p ?? [], {
				flow: s,
				indicator: "map-value-ind",
				next: m,
				offset: g.range[2],
				onError: i,
				parentIndent: r.indent,
				startOnNewline: !1
			});
			if (_.found) {
				if (!o && !h.found && n.options.strict) {
					if (p) for (let e of p) {
						if (e === _.found) break;
						if (e.type === "newline") {
							i(e, "MULTILINE_IMPLICIT_KEY", "Implicit keys of flow sequence pairs need to be on a single line");
							break;
						}
					}
					h.start < _.found.offset - 1024 && i(_.found, "KEY_OVER_1024_CHARS", "The : indicator must be at most 1024 chars after the start of an implicit flow sequence key");
				}
			} else m && ("source" in m && m.source?.[0] === ":" ? i(m, "MISSING_CHAR", `Missing space after : in ${s}`) : i(_.start, "MISSING_CHAR", `Missing , or : between ${s} items`));
			let v = m ? e(n, m, _, i) : _.found ? t(n, _.end, p, null, _, i) : null;
			v ? $c(m) && i(v.range, "BLOCK_IN_FLOW", Qc) : _.comment && (g.comment ? g.comment += "\n" + _.comment : g.comment = _.comment);
			let y = new Y(g, v);
			if (n.options.keepSourceTokens && (y.srcToken = l), o) {
				let e = c;
				qc(n, e.items, g) && i(a, "DUPLICATE_KEY", "Map keys must be unique"), e.items.push(y);
			} else {
				let e = new X(n.schema);
				e.flow = !0, e.items.push(y);
				let t = (v ?? g).range;
				e.range = [
					g.range[0],
					t[1],
					t[2]
				], c.items.push(e);
			}
			u = v ? v.range[2] : _.end;
		}
	}
	let d = o ? "}" : "]", [f, ...p] = r.end, m = u;
	if (f?.source === d) m = f.offset + f.source.length;
	else {
		let e = s[0].toUpperCase() + s.substring(1), t = l ? `${e} must end with a ${d}` : `${e} in block collection must be sufficiently indented and end with a ${d}`;
		i(u, l ? "MISSING_CHAR" : "BAD_INDENT", t), f && f.source.length !== 1 && p.unshift(f);
	}
	if (p.length > 0) {
		let e = Zc(p, m, n.options.strict, i);
		e.comment && (c.comment ? c.comment += "\n" + e.comment : c.comment = e.comment), c.range = [
			r.offset,
			m,
			e.offset
		];
	} else c.range = [
		r.offset,
		m,
		m
	];
	return c;
}
function tl(e, t, n, r, i, a) {
	let o = n.type === "block-map" ? Yc(e, t, n, r, a) : n.type === "block-seq" ? Xc(e, t, n, r, a) : el(e, t, n, r, a), s = o.constructor;
	return i === "!" || i === s.tagName ? (o.tag = s.tagName, o) : (i && (o.tag = i), o);
}
function nl(e, t, n, r, i) {
	let a = r.tag, o = a ? t.directives.tagName(a.source, (e) => i(a, "TAG_RESOLVE_FAILED", e)) : null;
	if (n.type === "block-seq") {
		let { anchor: e, newlineAfterProp: t } = r, n = e && a ? e.offset > a.offset ? e : a : e ?? a;
		n && (!t || t.offset < n.offset) && i(n, "MISSING_CHAR", "Missing newline after block sequence props");
	}
	let s = n.type === "block-map" ? "map" : n.type === "block-seq" ? "seq" : n.start.source === "{" ? "map" : "seq";
	if (!a || !o || o === "!" || o === X.tagName && s === "map" || o === Rs.tagName && s === "seq") return tl(e, t, n, i, o);
	let c = t.schema.tags.find((e) => e.tag === o && e.collection === s);
	if (!c) {
		let r = t.schema.knownTags[o];
		if (r?.collection === s) t.schema.tags.push(Object.assign({}, r, { default: !1 })), c = r;
		else return r ? i(a, "BAD_COLLECTION_TYPE", `${r.tag} used for ${s} collection, but expects ${r.collection ?? "scalar"}`, !0) : i(a, "TAG_RESOLVE_FAILED", `Unresolved tag: ${o}`, !0), tl(e, t, n, i, o);
	}
	let l = tl(e, t, n, i, o, c), u = c.resolve?.(l, (e) => i(a, "TAG_RESOLVE_FAILED", e), t.options) ?? l, d = H(u) ? u : new K(u);
	return d.range = l.range, d.tag = o, c?.format && (d.format = c.format), d;
}
function rl(e, t, n) {
	let r = t.offset, i = il(t, e.options.strict, n);
	if (!i) return {
		value: "",
		type: null,
		comment: "",
		range: [
			r,
			r,
			r
		]
	};
	let a = i.mode === ">" ? K.BLOCK_FOLDED : K.BLOCK_LITERAL, o = t.source ? al(t.source) : [], s = o.length;
	for (let e = o.length - 1; e >= 0; --e) {
		let t = o[e][1];
		if (t === "" || t === "\r") s = e;
		else break;
	}
	if (s === 0) {
		let e = i.chomp === "+" && o.length > 0 ? "\n".repeat(Math.max(1, o.length - 1)) : "", n = r + i.length;
		return t.source && (n += t.source.length), {
			value: e,
			type: a,
			comment: i.comment,
			range: [
				r,
				n,
				n
			]
		};
	}
	let c = t.indent + i.indent, l = t.offset + i.length, u = 0;
	for (let t = 0; t < s; ++t) {
		let [r, a] = o[t];
		if (a === "" || a === "\r") i.indent === 0 && r.length > c && (c = r.length);
		else {
			r.length < c && n(l + r.length, "MISSING_CHAR", "Block scalars with more-indented leading empty lines must use an explicit indentation indicator"), i.indent === 0 && (c = r.length), u = t, c === 0 && !e.atRoot && n(l, "BAD_INDENT", "Block scalar values in collections must be indented");
			break;
		}
		l += r.length + a.length + 1;
	}
	for (let e = o.length - 1; e >= s; --e) o[e][0].length > c && (s = e + 1);
	let d = "", f = "", p = !1;
	for (let e = 0; e < u; ++e) d += o[e][0].slice(c) + "\n";
	for (let e = u; e < s; ++e) {
		let [t, r] = o[e];
		l += t.length + r.length + 1;
		let s = r[r.length - 1] === "\r";
		/* istanbul ignore if already caught in lexer */
		if (s && (r = r.slice(0, -1)), r && t.length < c) {
			let e = `Block scalar lines must not be less indented than their ${i.indent ? "explicit indentation indicator" : "first line"}`;
			n(l - r.length - (s ? 2 : 1), "BAD_INDENT", e), t = "";
		}
		a === K.BLOCK_LITERAL ? (d += f + t.slice(c) + r, f = "\n") : t.length > c || r[0] === "	" ? (f === " " ? f = "\n" : !p && f === "\n" && (f = "\n\n"), d += f + t.slice(c) + r, f = "\n", p = !0) : r === "" ? f === "\n" ? d += "\n" : f = "\n" : (d += f + r, f = " ", p = !1);
	}
	switch (i.chomp) {
		case "-": break;
		case "+":
			for (let e = s; e < o.length; ++e) d += "\n" + o[e][0].slice(c);
			d[d.length - 1] !== "\n" && (d += "\n");
			break;
		default: d += "\n";
	}
	let m = r + i.length + t.source.length;
	return {
		value: d,
		type: a,
		comment: i.comment,
		range: [
			r,
			m,
			m
		]
	};
}
function il({ offset: e, props: t }, n, r) {
	/* istanbul ignore if should not happen */
	if (t[0].type !== "block-scalar-header") return r(t[0], "IMPOSSIBLE", "Block scalar header not found"), null;
	let { source: i } = t[0], a = i[0], o = 0, s = "", c = -1;
	for (let t = 1; t < i.length; ++t) {
		let n = i[t];
		if (!s && (n === "-" || n === "+")) s = n;
		else {
			let r = Number(n);
			!o && r ? o = r : c === -1 && (c = e + t);
		}
	}
	c !== -1 && r(c, "UNEXPECTED_TOKEN", `Block scalar header includes extra characters: ${i}`);
	let l = !1, u = "", d = i.length;
	for (let e = 1; e < t.length; ++e) {
		let i = t[e];
		switch (i.type) {
			case "space": l = !0;
			case "newline":
				d += i.source.length;
				break;
			case "comment":
				n && !l && r(i, "MISSING_CHAR", "Comments must be separated from other tokens by white space characters"), d += i.source.length, u = i.source.substring(1);
				break;
			case "error":
				r(i, "UNEXPECTED_TOKEN", i.message), d += i.source.length;
				break;
			/* istanbul ignore next should not happen */
			default: {
				r(i, "UNEXPECTED_TOKEN", `Unexpected token in block scalar header: ${i.type}`);
				let e = i.source;
				e && typeof e == "string" && (d += e.length);
			}
		}
	}
	return {
		mode: a,
		indent: o,
		chomp: s,
		comment: u,
		length: d
	};
}
function al(e) {
	let t = e.split(/\n( *)/), n = t[0], r = n.match(/^( *)/), i = [r?.[1] ? [r[1], n.slice(r[1].length)] : ["", n]];
	for (let e = 1; e < t.length; e += 2) i.push([t[e], t[e + 1]]);
	return i;
}
function ol(e, t, n) {
	let { offset: r, type: i, source: a, end: o } = e, s, c, l = (e, t, i) => n(r + e, t, i);
	switch (i) {
		case "scalar":
			s = K.PLAIN, c = sl(a, l);
			break;
		case "single-quoted-scalar":
			s = K.QUOTE_SINGLE, c = cl(a, l);
			break;
		case "double-quoted-scalar":
			s = K.QUOTE_DOUBLE, c = ul(a, l);
			break;
		/* istanbul ignore next should not happen */
		default: return n(e, "UNEXPECTED_TOKEN", `Expected a flow scalar value, but found: ${i}`), {
			value: "",
			type: null,
			comment: "",
			range: [
				r,
				r + a.length,
				r + a.length
			]
		};
	}
	let u = r + a.length, d = Zc(o, u, t, n);
	return {
		value: c,
		type: s,
		comment: d.comment,
		range: [
			r,
			u,
			d.offset
		]
	};
}
function sl(e, t) {
	let n = "";
	switch (e[0]) {
		/* istanbul ignore next should not happen */
		case "	":
			n = "a tab character";
			break;
		case ",":
			n = "flow indicator character ,";
			break;
		case "%":
			n = "directive indicator character %";
			break;
		case "|":
		case ">":
			n = `block scalar indicator ${e[0]}`;
			break;
		case "@":
		case "`":
			n = `reserved character ${e[0]}`;
			break;
	}
	return n && t(0, "BAD_SCALAR_START", `Plain value cannot start with ${n}`), ll(e);
}
function cl(e, t) {
	return (e[e.length - 1] !== "'" || e.length === 1) && t(e.length, "MISSING_CHAR", "Missing closing 'quote"), ll(e.slice(1, -1)).replace(/''/g, "'");
}
function ll(e) {
	let t, n;
	try {
		t = /* @__PURE__ */ RegExp("(.*?)(?<![ 	])[ 	]*\r?\n", "sy"), n = /* @__PURE__ */ RegExp("[ 	]*(.*?)(?:(?<![ 	])[ 	]*)?\r?\n", "sy");
	} catch {
		t = /(.*?)[ \t]*\r?\n/sy, n = /[ \t]*(.*?)[ \t]*\r?\n/sy;
	}
	let r = t.exec(e);
	if (!r) return e;
	let i = r[1], a = " ", o = t.lastIndex;
	for (n.lastIndex = o; r = n.exec(e);) r[1] === "" ? a === "\n" ? i += a : a = "\n" : (i += a + r[1], a = " "), o = n.lastIndex;
	let s = /[ \t]*(.*)/sy;
	return s.lastIndex = o, r = s.exec(e), i + a + (r?.[1] ?? "");
}
function ul(e, t) {
	let n = "";
	for (let r = 1; r < e.length - 1; ++r) {
		let i = e[r];
		if (!(i === "\r" && e[r + 1] === "\n")) if (i === "\n") {
			let { fold: t, offset: i } = dl(e, r);
			n += t, r = i;
		} else if (i === "\\") {
			let i = e[++r], a = fl[i];
			if (a) n += a;
			else if (i === "\n") for (i = e[r + 1]; i === " " || i === "	";) i = e[++r + 1];
			else if (i === "\r" && e[r + 1] === "\n") for (i = e[++r + 1]; i === " " || i === "	";) i = e[++r + 1];
			else if (i === "x" || i === "u" || i === "U") {
				let a = i === "x" ? 2 : i === "u" ? 4 : 8;
				n += pl(e, r + 1, a, t), r += a;
			} else {
				let i = e.substr(r - 1, 2);
				t(r - 1, "BAD_DQ_ESCAPE", `Invalid escape sequence ${i}`), n += i;
			}
		} else if (i === " " || i === "	") {
			let t = r, a = e[r + 1];
			for (; a === " " || a === "	";) a = e[++r + 1];
			a !== "\n" && !(a === "\r" && e[r + 2] === "\n") && (n += r > t ? e.slice(t, r + 1) : i);
		} else n += i;
	}
	return (e[e.length - 1] !== "\"" || e.length === 1) && t(e.length, "MISSING_CHAR", "Missing closing \"quote"), n;
}
function dl(e, t) {
	let n = "", r = e[t + 1];
	for (; (r === " " || r === "	" || r === "\n" || r === "\r") && !(r === "\r" && e[t + 2] !== "\n");) r === "\n" && (n += "\n"), t += 1, r = e[t + 1];
	return n ||= " ", {
		fold: n,
		offset: t
	};
}
var fl = {
	0: "\0",
	a: "\x07",
	b: "\b",
	e: "\x1B",
	f: "\f",
	n: "\n",
	r: "\r",
	t: "	",
	v: "\v",
	N: "",
	_: "\xA0",
	L: "\u2028",
	P: "\u2029",
	" ": " ",
	"\"": "\"",
	"/": "/",
	"\\": "\\",
	"	": "	"
};
function pl(e, t, n, r) {
	let i = e.substr(t, n), a = i.length === n && /^[0-9a-fA-F]+$/.test(i) ? parseInt(i, 16) : NaN;
	try {
		return String.fromCodePoint(a);
	} catch {
		let i = e.substr(t - 2, n + 2);
		return r(t - 2, "BAD_DQ_ESCAPE", `Invalid escape sequence ${i}`), i;
	}
}
function ml(e, t, n, r) {
	let { value: i, type: a, comment: o, range: s } = t.type === "block-scalar" ? rl(e, t, r) : ol(t, e.options.strict, r), c = n ? e.directives.tagName(n.source, (e) => r(n, "TAG_RESOLVE_FAILED", e)) : null, l;
	l = e.options.stringKeys && e.atKey ? e.schema[L] : c ? hl(e.schema, i, c, n, r) : t.type === "scalar" ? gl(e, i, t, r) : e.schema[L];
	let u;
	try {
		let a = l.resolve(i, (e) => r(n ?? t, "TAG_RESOLVE_FAILED", e), e.options);
		u = B(a) ? a : new K(a);
	} catch (e) {
		let a = e instanceof Error ? e.message : String(e);
		r(n ?? t, "TAG_RESOLVE_FAILED", a), u = new K(i);
	}
	return u.range = s, u.source = i, a && (u.type = a), c && (u.tag = c), l.format && (u.format = l.format), o && (u.comment = o), u;
}
function hl(e, t, n, r, i) {
	if (n === "!") return e[L];
	let a = [];
	for (let t of e.tags) if (!t.collection && t.tag === n) if (t.default && t.test) a.push(t);
	else return t;
	for (let e of a) if (e.test?.test(t)) return e;
	let o = e.knownTags[n];
	return o && !o.collection ? (e.tags.push(Object.assign({}, o, {
		default: !1,
		test: void 0
	})), o) : (i(r, "TAG_RESOLVE_FAILED", `Unresolved tag: ${n}`, n !== "tag:yaml.org,2002:str"), e[L]);
}
function gl({ atKey: e, directives: t, schema: n }, r, i, a) {
	let o = n.tags.find((t) => (t.default === !0 || e && t.default === "key") && t.test?.test(r)) || n[L];
	if (n.compat) {
		let e = n.compat.find((e) => e.default && e.test?.test(r)) ?? n[L];
		o.tag !== e.tag && a(i, "TAG_RESOLVE_FAILED", `Value may be parsed as either ${t.tagString(o.tag)} or ${t.tagString(e.tag)}`, !0);
	}
	return o;
}
function _l(e, t, n) {
	if (t) {
		n ??= t.length;
		for (let r = n - 1; r >= 0; --r) {
			let n = t[r];
			switch (n.type) {
				case "space":
				case "comment":
				case "newline":
					e -= n.source.length;
					continue;
			}
			for (n = t[++r]; n?.type === "space";) e += n.source.length, n = t[++r];
			break;
		}
	}
	return e;
}
var vl = {
	composeNode: yl,
	composeEmptyNode: bl
};
function yl(e, t, n, r) {
	let i = e.atKey, { spaceBefore: a, comment: o, anchor: s, tag: c } = n, l, u = !0;
	switch (t.type) {
		case "alias":
			l = xl(e, t, r), (s || c) && r(t, "ALIAS_PROPS", "An alias node must not specify any properties");
			break;
		case "scalar":
		case "single-quoted-scalar":
		case "double-quoted-scalar":
		case "block-scalar":
			l = ml(e, t, c, r), s && (l.anchor = s.source.substring(1));
			break;
		case "block-map":
		case "block-seq":
		case "flow-collection":
			try {
				l = nl(vl, e, t, n, r), s && (l.anchor = s.source.substring(1));
			} catch (e) {
				r(t, "RESOURCE_EXHAUSTION", e instanceof Error ? e.message : String(e));
			}
			break;
		default: r(t, "UNEXPECTED_TOKEN", t.type === "error" ? t.message : `Unsupported token (type: ${t.type})`), u = !1;
	}
	return l ??= bl(e, t.offset, void 0, null, n, r), s && l.anchor === "" && r(s, "BAD_ALIAS", "Anchor cannot be an empty string"), i && e.options.stringKeys && (!B(l) || typeof l.value != "string" || l.tag && l.tag !== "tag:yaml.org,2002:str") && r(c ?? t, "NON_STRING_KEY", "With stringKeys, all keys must be strings"), a && (l.spaceBefore = !0), o && (t.type === "scalar" && t.source === "" ? l.comment = o : l.commentBefore = o), e.options.keepSourceTokens && u && (l.srcToken = t), l;
}
function bl(e, t, n, r, { spaceBefore: i, comment: a, anchor: o, tag: s, end: c }, l) {
	let u = ml(e, {
		type: "scalar",
		offset: _l(t, n, r),
		indent: -1,
		source: ""
	}, s, l);
	return o && (u.anchor = o.source.substring(1), u.anchor === "" && l(o, "BAD_ALIAS", "Anchor cannot be an empty string")), i && (u.spaceBefore = !0), a && (u.comment = a, u.range[2] = c), u;
}
function xl({ options: e }, { offset: t, source: n, end: r }, i) {
	let a = new Ko(n.substring(1));
	a.source === "" && i(t, "BAD_ALIAS", "Alias cannot be an empty string"), a.source.endsWith(":") && i(t + n.length - 1, "BAD_ALIAS", "Alias ending in : is ambiguous", !0);
	let o = t + n.length, s = Zc(r, o, e.strict, i);
	return a.range = [
		t,
		o,
		s.offset
	], s.comment && (a.comment = s.comment), a;
}
function Sl(e, t, { offset: n, start: r, value: i, end: a }, o) {
	let s = new Rc(void 0, Object.assign({ _directives: t }, e)), c = {
		atKey: !1,
		atRoot: !0,
		directives: s.directives,
		options: s.options,
		schema: s.schema
	}, l = Wc(r, {
		indicator: "doc-start",
		next: i ?? a?.[0],
		offset: n,
		onError: o,
		parentIndent: 0,
		startOnNewline: !0
	});
	l.found && (s.directives.docStart = !0, i && (i.type === "block-map" || i.type === "block-seq") && !l.hasNewline && o(l.end, "MISSING_CHAR", "Block collection cannot start on same line with directives-end marker")), s.contents = i ? yl(c, i, l, o) : bl(c, l.end, r, null, l, o);
	let u = s.contents.range[2], d = Zc(a, u, !1, o);
	return d.comment && (s.comment = d.comment), s.range = [
		n,
		u,
		d.offset
	], s;
}
function Cl(e) {
	if (typeof e == "number") return [e, e + 1];
	if (Array.isArray(e)) return e.length === 2 ? e : [e[0], e[1]];
	let { offset: t, source: n } = e;
	return [t, t + (typeof n == "string" ? n.length : 1)];
}
function wl(e) {
	let t = "", n = !1, r = !1;
	for (let i = 0; i < e.length; ++i) {
		let a = e[i];
		switch (a[0]) {
			case "#":
				t += (t === "" ? "" : r ? "\n\n" : "\n") + (a.substring(1) || " "), n = !0, r = !1;
				break;
			case "%":
				e[i + 1]?.[0] !== "#" && (i += 1), n = !1;
				break;
			default: n || (r = !0), n = !1;
		}
	}
	return {
		comment: t,
		afterEmptyLine: r
	};
}
var Tl = class {
	constructor(e = {}) {
		this.doc = null, this.atDirectives = !1, this.prelude = [], this.errors = [], this.warnings = [], this.onError = (e, t, n, r) => {
			let i = Cl(e);
			r ? this.warnings.push(new Hc(i, t, n)) : this.errors.push(new Vc(i, t, n));
		}, this.directives = new zo({ version: e.version || "1.2" }), this.options = e;
	}
	decorate(e, t) {
		let { comment: n, afterEmptyLine: r } = wl(this.prelude);
		if (n) {
			let i = e.contents;
			if (t) e.comment = e.comment ? `${e.comment}\n${n}` : n;
			else if (r || e.directives.docStart || !i) e.commentBefore = n;
			else if (V(i) && !i.flow && i.items.length > 0) {
				let e = i.items[0];
				z(e) && (e = e.key);
				let t = e.commentBefore;
				e.commentBefore = t ? `${n}\n${t}` : n;
			} else {
				let e = i.commentBefore;
				i.commentBefore = e ? `${n}\n${e}` : n;
			}
		}
		if (t) {
			for (let t = 0; t < this.errors.length; ++t) e.errors.push(this.errors[t]);
			for (let t = 0; t < this.warnings.length; ++t) e.warnings.push(this.warnings[t]);
		} else e.errors = this.errors, e.warnings = this.warnings;
		this.prelude = [], this.errors = [], this.warnings = [];
	}
	streamInfo() {
		return {
			comment: wl(this.prelude).comment,
			directives: this.directives,
			errors: this.errors,
			warnings: this.warnings
		};
	}
	*compose(e, t = !1, n = -1) {
		for (let t of e) yield* this.next(t);
		yield* this.end(t, n);
	}
	*next(e) {
		switch (e.type) {
			case "directive":
				this.directives.add(e.source, (t, n, r) => {
					let i = Cl(e);
					i[0] += t, this.onError(i, "BAD_DIRECTIVE", n, r);
				}), this.prelude.push(e.source), this.atDirectives = !0;
				break;
			case "document": {
				let t = Sl(this.options, this.directives, e, this.onError);
				this.atDirectives && !t.directives.docStart && this.onError(e, "MISSING_CHAR", "Missing directives-end/doc-start indicator line"), this.decorate(t, !1), this.doc && (yield this.doc), this.doc = t, this.atDirectives = !1;
				break;
			}
			case "byte-order-mark":
			case "space": break;
			case "comment":
			case "newline":
				this.prelude.push(e.source);
				break;
			case "error": {
				let t = e.source ? `${e.message}: ${JSON.stringify(e.source)}` : e.message, n = new Vc(Cl(e), "UNEXPECTED_TOKEN", t);
				this.atDirectives || !this.doc ? this.errors.push(n) : this.doc.errors.push(n);
				break;
			}
			case "doc-end": {
				if (!this.doc) {
					this.errors.push(new Vc(Cl(e), "UNEXPECTED_TOKEN", "Unexpected doc-end without preceding document"));
					break;
				}
				this.doc.directives.docEnd = !0;
				let t = Zc(e.end, e.offset + e.source.length, this.doc.options.strict, this.onError);
				if (this.decorate(this.doc, !0), t.comment) {
					let e = this.doc.comment;
					this.doc.comment = e ? `${e}\n${t.comment}` : t.comment;
				}
				this.doc.range[2] = t.offset;
				break;
			}
			default: this.errors.push(new Vc(Cl(e), "UNEXPECTED_TOKEN", `Unsupported token ${e.type}`));
		}
	}
	*end(e = !1, t = -1) {
		if (this.doc) this.decorate(this.doc, !0), yield this.doc, this.doc = null;
		else if (e) {
			let e = new Rc(void 0, Object.assign({ _directives: this.directives }, this.options));
			this.atDirectives && this.onError(t, "MISSING_CHAR", "Missing directives-end indicator line"), e.range = [
				0,
				t,
				t
			], this.decorate(e, !1), yield e;
		}
	}
}, El = Symbol("break visit"), Dl = Symbol("skip children"), Ol = Symbol("remove item");
function kl(e, t) {
	"type" in e && e.type === "document" && (e = {
		start: e.start,
		value: e.value
	}), Al(Object.freeze([]), e, t);
}
kl.BREAK = El, kl.SKIP = Dl, kl.REMOVE = Ol, kl.itemAtPath = (e, t) => {
	let n = e;
	for (let [e, r] of t) {
		let t = n?.[e];
		if (t && "items" in t) n = t.items[r];
		else return;
	}
	return n;
}, kl.parentCollection = (e, t) => {
	let n = kl.itemAtPath(e, t.slice(0, -1)), r = t[t.length - 1][0], i = n?.[r];
	if (i && "items" in i) return i;
	throw Error("Parent collection not found");
};
function Al(e, t, n) {
	let r = n(t, e);
	if (typeof r == "symbol") return r;
	for (let i of ["key", "value"]) {
		let a = t[i];
		if (a && "items" in a) {
			for (let t = 0; t < a.items.length; ++t) {
				let r = Al(Object.freeze(e.concat([[i, t]])), a.items[t], n);
				if (typeof r == "number") t = r - 1;
				else if (r === El) return El;
				else r === Ol && (a.items.splice(t, 1), --t);
			}
			typeof r == "function" && i === "key" && (r = r(t, e));
		}
	}
	return typeof r == "function" ? r(t, e) : r;
}
function jl(e) {
	switch (e) {
		case "﻿": return "byte-order-mark";
		case "": return "doc-mode";
		case "": return "flow-error-end";
		case "": return "scalar";
		case "---": return "doc-start";
		case "...": return "doc-end";
		case "":
		case "\n":
		case "\r\n": return "newline";
		case "-": return "seq-item-ind";
		case "?": return "explicit-key-ind";
		case ":": return "map-value-ind";
		case "{": return "flow-map-start";
		case "}": return "flow-map-end";
		case "[": return "flow-seq-start";
		case "]": return "flow-seq-end";
		case ",": return "comma";
	}
	switch (e[0]) {
		case " ":
		case "	": return "space";
		case "#": return "comment";
		case "%": return "directive-line";
		case "*": return "alias";
		case "&": return "anchor";
		case "!": return "tag";
		case "'": return "single-quoted-scalar";
		case "\"": return "double-quoted-scalar";
		case "|":
		case ">": return "block-scalar-header";
	}
	return null;
}
function Q(e) {
	switch (e) {
		case void 0:
		case " ":
		case "\n":
		case "\r":
		case "	": return !0;
		default: return !1;
	}
}
var Ml = /* @__PURE__ */ new Set("0123456789ABCDEFabcdef"), Nl = /* @__PURE__ */ new Set("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-#;/?:@&=+$_.!~*'()"), Pl = /* @__PURE__ */ new Set(",[]{}"), Fl = /* @__PURE__ */ new Set(" ,[]{}\n\r	"), Il = (e) => !e || Fl.has(e), Ll = class {
	constructor() {
		this.atEnd = !1, this.blockScalarIndent = -1, this.blockScalarKeep = !1, this.buffer = "", this.flowKey = !1, this.flowLevel = 0, this.indentNext = 0, this.indentValue = 0, this.lineEndPos = null, this.next = null, this.pos = 0;
	}
	*lex(e, t = !1) {
		if (e) {
			if (typeof e != "string") throw TypeError("source is not a string");
			this.buffer = this.buffer ? this.buffer + e : e, this.lineEndPos = null;
		}
		this.atEnd = !t;
		let n = this.next ?? "stream";
		for (; n && (t || this.hasChars(1));) n = yield* this.parseNext(n);
	}
	atLineEnd() {
		let e = this.pos, t = this.buffer[e];
		for (; t === " " || t === "	";) t = this.buffer[++e];
		return !t || t === "#" || t === "\n" ? !0 : t === "\r" ? this.buffer[e + 1] === "\n" : !1;
	}
	charAt(e) {
		return this.buffer[this.pos + e];
	}
	continueScalar(e) {
		let t = this.buffer[e];
		if (this.indentNext > 0) {
			let n = 0;
			for (; t === " ";) t = this.buffer[++n + e];
			if (t === "\r") {
				let t = this.buffer[n + e + 1];
				if (t === "\n" || !t && !this.atEnd) return e + n + 1;
			}
			return t === "\n" || n >= this.indentNext || !t && !this.atEnd ? e + n : -1;
		}
		if (t === "-" || t === ".") {
			let t = this.buffer.substr(e, 3);
			if ((t === "---" || t === "...") && Q(this.buffer[e + 3])) return -1;
		}
		return e;
	}
	getLine() {
		let e = this.lineEndPos;
		return (typeof e != "number" || e !== -1 && e < this.pos) && (e = this.buffer.indexOf("\n", this.pos), this.lineEndPos = e), e === -1 ? this.atEnd ? this.buffer.substring(this.pos) : null : (this.buffer[e - 1] === "\r" && --e, this.buffer.substring(this.pos, e));
	}
	hasChars(e) {
		return this.pos + e <= this.buffer.length;
	}
	setNext(e) {
		return this.buffer = this.buffer.substring(this.pos), this.pos = 0, this.lineEndPos = null, this.next = e, null;
	}
	peek(e) {
		return this.buffer.substr(this.pos, e);
	}
	*parseNext(e) {
		switch (e) {
			case "stream": return yield* this.parseStream();
			case "line-start": return yield* this.parseLineStart();
			case "block-start": return yield* this.parseBlockStart();
			case "doc": return yield* this.parseDocument();
			case "flow": return yield* this.parseFlowCollection();
			case "quoted-scalar": return yield* this.parseQuotedScalar();
			case "block-scalar": return yield* this.parseBlockScalar();
			case "plain-scalar": return yield* this.parsePlainScalar();
		}
	}
	*parseStream() {
		let e = this.getLine();
		if (e === null) return this.setNext("stream");
		if (e[0] === "﻿" && (yield* this.pushCount(1), e = e.substring(1)), e[0] === "%") {
			let t = e.length, n = e.indexOf("#");
			for (; n !== -1;) {
				let r = e[n - 1];
				if (r === " " || r === "	") {
					t = n - 1;
					break;
				} else n = e.indexOf("#", n + 1);
			}
			for (;;) {
				let n = e[t - 1];
				if (n === " " || n === "	") --t;
				else break;
			}
			let r = (yield* this.pushCount(t)) + (yield* this.pushSpaces(!0));
			return yield* this.pushCount(e.length - r), this.pushNewline(), "stream";
		}
		if (this.atLineEnd()) {
			let t = yield* this.pushSpaces(!0);
			return yield* this.pushCount(e.length - t), yield* this.pushNewline(), "stream";
		}
		return yield "", yield* this.parseLineStart();
	}
	*parseLineStart() {
		let e = this.charAt(0);
		if (!e && !this.atEnd) return this.setNext("line-start");
		if (e === "-" || e === ".") {
			if (!this.atEnd && !this.hasChars(4)) return this.setNext("line-start");
			let e = this.peek(3);
			if ((e === "---" || e === "...") && Q(this.charAt(3))) return yield* this.pushCount(3), this.indentValue = 0, this.indentNext = 0, e === "---" ? "doc" : "stream";
		}
		return this.indentValue = yield* this.pushSpaces(!1), this.indentNext > this.indentValue && !Q(this.charAt(1)) && (this.indentNext = this.indentValue), yield* this.parseBlockStart();
	}
	*parseBlockStart() {
		let [e, t] = this.peek(2);
		if (!t && !this.atEnd) return this.setNext("block-start");
		if ((e === "-" || e === "?" || e === ":") && Q(t)) {
			let e = (yield* this.pushCount(1)) + (yield* this.pushSpaces(!0));
			return this.indentNext = this.indentValue + 1, this.indentValue += e, "block-start";
		}
		return "doc";
	}
	*parseDocument() {
		yield* this.pushSpaces(!0);
		let e = this.getLine();
		if (e === null) return this.setNext("doc");
		let t = yield* this.pushIndicators();
		switch (e[t]) {
			case "#": yield* this.pushCount(e.length - t);
			case void 0: return yield* this.pushNewline(), yield* this.parseLineStart();
			case "{":
			case "[": return yield* this.pushCount(1), this.flowKey = !1, this.flowLevel = 1, "flow";
			case "}":
			case "]": return yield* this.pushCount(1), "doc";
			case "*": return yield* this.pushUntil(Il), "doc";
			case "\"":
			case "'": return yield* this.parseQuotedScalar();
			case "|":
			case ">": return t += yield* this.parseBlockScalarHeader(), t += yield* this.pushSpaces(!0), yield* this.pushCount(e.length - t), yield* this.pushNewline(), yield* this.parseBlockScalar();
			default: return yield* this.parsePlainScalar();
		}
	}
	*parseFlowCollection() {
		let e, t, n = -1;
		do
			e = yield* this.pushNewline(), e > 0 ? (t = yield* this.pushSpaces(!1), this.indentValue = n = t) : t = 0, t += yield* this.pushSpaces(!0);
		while (e + t > 0);
		let r = this.getLine();
		if (r === null) return this.setNext("flow");
		if ((n !== -1 && n < this.indentNext && r[0] !== "#" || n === 0 && (r.startsWith("---") || r.startsWith("...")) && Q(r[3])) && !(n === this.indentNext - 1 && this.flowLevel === 1 && (r[0] === "]" || r[0] === "}"))) return this.flowLevel = 0, yield "", yield* this.parseLineStart();
		let i = 0;
		for (; r[i] === ",";) i += yield* this.pushCount(1), i += yield* this.pushSpaces(!0), this.flowKey = !1;
		switch (i += yield* this.pushIndicators(), r[i]) {
			case void 0: return "flow";
			case "#": return yield* this.pushCount(r.length - i), "flow";
			case "{":
			case "[": return yield* this.pushCount(1), this.flowKey = !1, this.flowLevel += 1, "flow";
			case "}":
			case "]": return yield* this.pushCount(1), this.flowKey = !0, --this.flowLevel, this.flowLevel ? "flow" : "doc";
			case "*": return yield* this.pushUntil(Il), "flow";
			case "\"":
			case "'": return this.flowKey = !0, yield* this.parseQuotedScalar();
			case ":": {
				let e = this.charAt(1);
				if (this.flowKey || Q(e) || e === ",") return this.flowKey = !1, yield* this.pushCount(1), yield* this.pushSpaces(!0), "flow";
			}
			default: return this.flowKey = !1, yield* this.parsePlainScalar();
		}
	}
	*parseQuotedScalar() {
		let e = this.charAt(0), t = this.buffer.indexOf(e, this.pos + 1);
		if (e === "'") for (; t !== -1 && this.buffer[t + 1] === "'";) t = this.buffer.indexOf("'", t + 2);
		else for (; t !== -1;) {
			let e = 0;
			for (; this.buffer[t - 1 - e] === "\\";) e += 1;
			if (e % 2 == 0) break;
			t = this.buffer.indexOf("\"", t + 1);
		}
		let n = this.buffer.substring(0, t), r = n.indexOf("\n", this.pos);
		if (r !== -1) {
			for (; r !== -1;) {
				let e = this.continueScalar(r + 1);
				if (e === -1) break;
				r = n.indexOf("\n", e);
			}
			r !== -1 && (t = r - (n[r - 1] === "\r" ? 2 : 1));
		}
		if (t === -1) {
			if (!this.atEnd) return this.setNext("quoted-scalar");
			t = this.buffer.length;
		}
		return yield* this.pushToIndex(t + 1, !1), this.flowLevel ? "flow" : "doc";
	}
	*parseBlockScalarHeader() {
		this.blockScalarIndent = -1, this.blockScalarKeep = !1;
		let e = this.pos;
		for (;;) {
			let t = this.buffer[++e];
			if (t === "+") this.blockScalarKeep = !0;
			else if (t > "0" && t <= "9") this.blockScalarIndent = Number(t) - 1;
			else if (t !== "-") break;
		}
		return yield* this.pushUntil((e) => Q(e) || e === "#");
	}
	*parseBlockScalar() {
		let e = this.pos - 1, t = 0, n;
		loop: for (let r = this.pos; n = this.buffer[r]; ++r) switch (n) {
			case " ":
				t += 1;
				break;
			case "\n":
				e = r, t = 0;
				break;
			case "\r": {
				let e = this.buffer[r + 1];
				if (!e && !this.atEnd) return this.setNext("block-scalar");
				if (e === "\n") break;
			}
			default: break loop;
		}
		if (!n && !this.atEnd) return this.setNext("block-scalar");
		if (t >= this.indentNext) {
			this.blockScalarIndent === -1 ? this.indentNext = t : this.indentNext = this.blockScalarIndent + (this.indentNext === 0 ? 1 : this.indentNext);
			do {
				let t = this.continueScalar(e + 1);
				if (t === -1) break;
				e = this.buffer.indexOf("\n", t);
			} while (e !== -1);
			if (e === -1) {
				if (!this.atEnd) return this.setNext("block-scalar");
				e = this.buffer.length;
			}
		}
		let r = e + 1;
		for (n = this.buffer[r]; n === " ";) n = this.buffer[++r];
		if (n === "	") {
			for (; n === "	" || n === " " || n === "\r" || n === "\n";) n = this.buffer[++r];
			e = r - 1;
		} else if (!this.blockScalarKeep) do {
			let n = e - 1, r = this.buffer[n];
			r === "\r" && (r = this.buffer[--n]);
			let i = n;
			for (; r === " ";) r = this.buffer[--n];
			if (r === "\n" && n >= this.pos && n + 1 + t > i) e = n;
			else break;
		} while (!0);
		return yield "", yield* this.pushToIndex(e + 1, !0), yield* this.parseLineStart();
	}
	*parsePlainScalar() {
		let e = this.flowLevel > 0, t = this.pos - 1, n = this.pos - 1, r;
		for (; r = this.buffer[++n];) if (r === ":") {
			let r = this.buffer[n + 1];
			if (Q(r) || e && Pl.has(r)) break;
			t = n;
		} else if (Q(r)) {
			let i = this.buffer[n + 1];
			if (r === "\r" && (i === "\n" ? (n += 1, r = "\n", i = this.buffer[n + 1]) : t = n), i === "#" || e && Pl.has(i)) break;
			if (r === "\n") {
				let e = this.continueScalar(n + 1);
				if (e === -1) break;
				n = Math.max(n, e - 2);
			}
		} else {
			if (e && Pl.has(r)) break;
			t = n;
		}
		return !r && !this.atEnd ? this.setNext("plain-scalar") : (yield "", yield* this.pushToIndex(t + 1, !0), e ? "flow" : "doc");
	}
	*pushCount(e) {
		return e > 0 ? (yield this.buffer.substr(this.pos, e), this.pos += e, e) : 0;
	}
	*pushToIndex(e, t) {
		let n = this.buffer.slice(this.pos, e);
		return n ? (yield n, this.pos += n.length, n.length) : (t && (yield ""), 0);
	}
	*pushIndicators() {
		let e = 0;
		loop: for (;;) {
			switch (this.charAt(0)) {
				case "!":
					e += yield* this.pushTag(), e += yield* this.pushSpaces(!0);
					continue loop;
				case "&":
					e += yield* this.pushUntil(Il), e += yield* this.pushSpaces(!0);
					continue loop;
				case "-":
				case "?":
				case ":": {
					let t = this.flowLevel > 0, n = this.charAt(1);
					if (Q(n) || t && Pl.has(n)) {
						t ? this.flowKey &&= !1 : this.indentNext = this.indentValue + 1, e += yield* this.pushCount(1), e += yield* this.pushSpaces(!0);
						continue loop;
					}
				}
			}
			break loop;
		}
		return e;
	}
	*pushTag() {
		if (this.charAt(1) === "<") {
			let e = this.pos + 2, t = this.buffer[e];
			for (; !Q(t) && t !== ">";) t = this.buffer[++e];
			return yield* this.pushToIndex(t === ">" ? e + 1 : e, !1);
		} else {
			let e = this.pos + 1, t = this.buffer[e];
			for (; t;) if (Nl.has(t)) t = this.buffer[++e];
			else if (t === "%" && Ml.has(this.buffer[e + 1]) && Ml.has(this.buffer[e + 2])) t = this.buffer[e += 3];
			else break;
			return yield* this.pushToIndex(e, !1);
		}
	}
	*pushNewline() {
		let e = this.buffer[this.pos];
		return e === "\n" ? yield* this.pushCount(1) : e === "\r" && this.charAt(1) === "\n" ? yield* this.pushCount(2) : 0;
	}
	*pushSpaces(e) {
		let t = this.pos - 1, n;
		do
			n = this.buffer[++t];
		while (n === " " || e && n === "	");
		let r = t - this.pos;
		return r > 0 && (yield this.buffer.substr(this.pos, r), this.pos = t), r;
	}
	*pushUntil(e) {
		let t = this.pos, n = this.buffer[t];
		for (; !e(n);) n = this.buffer[++t];
		return yield* this.pushToIndex(t, !1);
	}
}, Rl = class {
	constructor() {
		this.lineStarts = [], this.addNewLine = (e) => this.lineStarts.push(e), this.linePos = (e) => {
			let t = 0, n = this.lineStarts.length;
			for (; t < n;) {
				let r = t + n >> 1;
				this.lineStarts[r] < e ? t = r + 1 : n = r;
			}
			if (this.lineStarts[t] === e) return {
				line: t + 1,
				col: 1
			};
			if (t === 0) return {
				line: 0,
				col: e
			};
			let r = this.lineStarts[t - 1];
			return {
				line: t,
				col: e - r + 1
			};
		};
	}
};
function $(e, t) {
	for (let n = 0; n < e.length; ++n) if (e[n].type === t) return !0;
	return !1;
}
function zl(e) {
	for (let t = 0; t < e.length; ++t) switch (e[t].type) {
		case "space":
		case "comment":
		case "newline": break;
		default: return t;
	}
	return -1;
}
function Bl(e) {
	switch (e?.type) {
		case "alias":
		case "scalar":
		case "single-quoted-scalar":
		case "double-quoted-scalar":
		case "flow-collection": return !0;
		default: return !1;
	}
}
function Vl(e) {
	switch (e.type) {
		case "document": return e.start;
		case "block-map": {
			let t = e.items[e.items.length - 1];
			return t.sep ?? t.start;
		}
		case "block-seq": return e.items[e.items.length - 1].start;
		/* istanbul ignore next should not happen */
		default: return [];
	}
}
function Hl(e) {
	if (e.length === 0) return [];
	let t = e.length;
	loop: for (; --t >= 0;) switch (e[t].type) {
		case "doc-start":
		case "explicit-key-ind":
		case "map-value-ind":
		case "seq-item-ind":
		case "newline": break loop;
	}
	for (; e[++t]?.type === "space";);
	return e.splice(t, e.length);
}
function Ul(e, t) {
	if (t.length < 1e5) Array.prototype.push.apply(e, t);
	else for (let n = 0; n < t.length; ++n) e.push(t[n]);
}
function Wl(e) {
	if (e.start.type === "flow-seq-start") for (let t of e.items) t.sep && !t.value && !$(t.start, "explicit-key-ind") && !$(t.sep, "map-value-ind") && (t.key && (t.value = t.key), delete t.key, Bl(t.value) ? t.value.end ? Ul(t.value.end, t.sep) : t.value.end = t.sep : Ul(t.start, t.sep), delete t.sep);
}
var Gl = class {
	constructor(e) {
		this.atNewLine = !0, this.atScalar = !1, this.indent = 0, this.offset = 0, this.onKeyLine = !1, this.stack = [], this.source = "", this.type = "", this.lexer = new Ll(), this.onNewLine = e;
	}
	*parse(e, t = !1) {
		this.onNewLine && this.offset === 0 && this.onNewLine(0);
		for (let n of this.lexer.lex(e, t)) yield* this.next(n);
		t || (yield* this.end());
	}
	*next(e) {
		if (this.source = e, this.atScalar) {
			this.atScalar = !1, yield* this.step(), this.offset += e.length;
			return;
		}
		let t = jl(e);
		if (!t) {
			let t = `Not a YAML token: ${e}`;
			yield* this.pop({
				type: "error",
				offset: this.offset,
				message: t,
				source: e
			}), this.offset += e.length;
		} else if (t === "scalar") this.atNewLine = !1, this.atScalar = !0, this.type = "scalar";
		else {
			switch (this.type = t, yield* this.step(), t) {
				case "newline":
					this.atNewLine = !0, this.indent = 0, this.onNewLine && this.onNewLine(this.offset + e.length);
					break;
				case "space":
					this.atNewLine && e[0] === " " && (this.indent += e.length);
					break;
				case "explicit-key-ind":
				case "map-value-ind":
				case "seq-item-ind":
					this.atNewLine && (this.indent += e.length);
					break;
				case "doc-mode":
				case "flow-error-end": return;
				default: this.atNewLine = !1;
			}
			this.offset += e.length;
		}
	}
	*end() {
		for (; this.stack.length > 0;) yield* this.pop();
	}
	get sourceToken() {
		return {
			type: this.type,
			offset: this.offset,
			indent: this.indent,
			source: this.source
		};
	}
	*step() {
		let e = this.peek(1);
		if (this.type === "doc-end" && e?.type !== "doc-end") {
			for (; this.stack.length > 0;) yield* this.pop();
			this.stack.push({
				type: "doc-end",
				offset: this.offset,
				source: this.source
			});
			return;
		}
		if (!e) return yield* this.stream();
		switch (e.type) {
			case "document": return yield* this.document(e);
			case "alias":
			case "scalar":
			case "single-quoted-scalar":
			case "double-quoted-scalar": return yield* this.scalar(e);
			case "block-scalar": return yield* this.blockScalar(e);
			case "block-map": return yield* this.blockMap(e);
			case "block-seq": return yield* this.blockSequence(e);
			case "flow-collection": return yield* this.flowCollection(e);
			case "doc-end": return yield* this.documentEnd(e);
		}
		/* istanbul ignore next should not happen */
		yield* this.pop();
	}
	peek(e) {
		return this.stack[this.stack.length - e];
	}
	*pop(e) {
		let t = e ?? this.stack.pop();
		/* istanbul ignore if should not happen */
		if (!t) yield {
			type: "error",
			offset: this.offset,
			source: "",
			message: "Tried to pop an empty stack"
		};
		else if (this.stack.length === 0) yield t;
		else {
			let e = this.peek(1);
			switch (t.type === "block-scalar" ? t.indent = "indent" in e ? e.indent : 0 : t.type === "flow-collection" && e.type === "document" && (t.indent = 0), t.type === "flow-collection" && Wl(t), e.type) {
				case "document":
					e.value = t;
					break;
				case "block-scalar":
					e.props.push(t);
					break;
				case "block-map": {
					let n = e.items[e.items.length - 1];
					if (n.value) {
						e.items.push({
							start: [],
							key: t,
							sep: []
						}), this.onKeyLine = !0;
						return;
					} else if (n.sep) n.value = t;
					else {
						Object.assign(n, {
							key: t,
							sep: []
						}), this.onKeyLine = !n.explicitKey;
						return;
					}
					break;
				}
				case "block-seq": {
					let n = e.items[e.items.length - 1];
					n.value ? e.items.push({
						start: [],
						value: t
					}) : n.value = t;
					break;
				}
				case "flow-collection": {
					let n = e.items[e.items.length - 1];
					!n || n.value ? e.items.push({
						start: [],
						key: t,
						sep: []
					}) : n.sep ? n.value = t : Object.assign(n, {
						key: t,
						sep: []
					});
					return;
				}
				/* istanbul ignore next should not happen */
				default: yield* this.pop(), yield* this.pop(t);
			}
			if ((e.type === "document" || e.type === "block-map" || e.type === "block-seq") && (t.type === "block-map" || t.type === "block-seq")) {
				let n = t.items[t.items.length - 1];
				n && !n.sep && !n.value && n.start.length > 0 && zl(n.start) === -1 && (t.indent === 0 || n.start.every((e) => e.type !== "comment" || e.indent < t.indent)) && (e.type === "document" ? e.end = n.start : e.items.push({ start: n.start }), t.items.splice(-1, 1));
			}
		}
	}
	*stream() {
		switch (this.type) {
			case "directive-line":
				yield {
					type: "directive",
					offset: this.offset,
					source: this.source
				};
				return;
			case "byte-order-mark":
			case "space":
			case "comment":
			case "newline":
				yield this.sourceToken;
				return;
			case "doc-mode":
			case "doc-start": {
				let e = {
					type: "document",
					offset: this.offset,
					start: []
				};
				this.type === "doc-start" && e.start.push(this.sourceToken), this.stack.push(e);
				return;
			}
		}
		yield {
			type: "error",
			offset: this.offset,
			message: `Unexpected ${this.type} token in YAML stream`,
			source: this.source
		};
	}
	*document(e) {
		if (e.value) return yield* this.lineEnd(e);
		switch (this.type) {
			case "doc-start":
				zl(e.start) === -1 ? e.start.push(this.sourceToken) : (yield* this.pop(), yield* this.step());
				return;
			case "anchor":
			case "tag":
			case "space":
			case "comment":
			case "newline":
				e.start.push(this.sourceToken);
				return;
		}
		let t = this.startBlockValue(e);
		t ? this.stack.push(t) : yield {
			type: "error",
			offset: this.offset,
			message: `Unexpected ${this.type} token in YAML document`,
			source: this.source
		};
	}
	*scalar(e) {
		if (this.type === "map-value-ind") {
			let t = Hl(Vl(this.peek(2))), n;
			e.end ? (n = e.end, n.push(this.sourceToken), delete e.end) : n = [this.sourceToken];
			let r = {
				type: "block-map",
				offset: e.offset,
				indent: e.indent,
				items: [{
					start: t,
					key: e,
					sep: n
				}]
			};
			this.onKeyLine = !0, this.stack[this.stack.length - 1] = r;
		} else yield* this.lineEnd(e);
	}
	*blockScalar(e) {
		switch (this.type) {
			case "space":
			case "comment":
			case "newline":
				e.props.push(this.sourceToken);
				return;
			case "scalar":
				if (e.source = this.source, this.atNewLine = !0, this.indent = 0, this.onNewLine) {
					let e = this.source.indexOf("\n") + 1;
					for (; e !== 0;) this.onNewLine(this.offset + e), e = this.source.indexOf("\n", e) + 1;
				}
				yield* this.pop();
				break;
			/* istanbul ignore next should not happen */
			default: yield* this.pop(), yield* this.step();
		}
	}
	*blockMap(e) {
		let t = e.items[e.items.length - 1];
		switch (this.type) {
			case "newline":
				if (this.onKeyLine = !1, t.value) {
					let n = "end" in t.value ? t.value.end : void 0;
					(Array.isArray(n) ? n[n.length - 1] : void 0)?.type === "comment" ? n?.push(this.sourceToken) : e.items.push({ start: [this.sourceToken] });
				} else t.sep ? t.sep.push(this.sourceToken) : t.start.push(this.sourceToken);
				return;
			case "space":
			case "comment":
				if (t.value) e.items.push({ start: [this.sourceToken] });
				else if (t.sep) t.sep.push(this.sourceToken);
				else {
					if (this.atIndentedComment(t.start, e.indent)) {
						let n = e.items[e.items.length - 2]?.value?.end;
						if (Array.isArray(n)) {
							Ul(n, t.start), n.push(this.sourceToken), e.items.pop();
							return;
						}
					}
					t.start.push(this.sourceToken);
				}
				return;
		}
		if (this.indent >= e.indent) {
			let n = !this.onKeyLine && this.indent === e.indent, r = n && (t.sep || t.explicitKey) && this.type !== "seq-item-ind", i = [];
			if (r && t.sep && !t.value) {
				let n = [];
				for (let r = 0; r < t.sep.length; ++r) {
					let i = t.sep[r];
					switch (i.type) {
						case "newline":
							n.push(r);
							break;
						case "space": break;
						case "comment":
							i.indent > e.indent && (n.length = 0);
							break;
						default: n.length = 0;
					}
				}
				n.length >= 2 && (i = t.sep.splice(n[1]));
			}
			switch (this.type) {
				case "anchor":
				case "tag":
					r || t.value ? (i.push(this.sourceToken), e.items.push({ start: i }), this.onKeyLine = !0) : t.sep ? t.sep.push(this.sourceToken) : t.start.push(this.sourceToken);
					return;
				case "explicit-key-ind":
					!t.sep && !t.explicitKey ? (t.start.push(this.sourceToken), t.explicitKey = !0) : r || t.value ? (i.push(this.sourceToken), e.items.push({
						start: i,
						explicitKey: !0
					})) : this.stack.push({
						type: "block-map",
						offset: this.offset,
						indent: this.indent,
						items: [{
							start: [this.sourceToken],
							explicitKey: !0
						}]
					}), this.onKeyLine = !0;
					return;
				case "map-value-ind":
					if (t.explicitKey) if (!t.sep) if ($(t.start, "newline")) Object.assign(t, {
						key: null,
						sep: [this.sourceToken]
					});
					else {
						let e = Hl(t.start);
						this.stack.push({
							type: "block-map",
							offset: this.offset,
							indent: this.indent,
							items: [{
								start: e,
								key: null,
								sep: [this.sourceToken]
							}]
						});
					}
					else if (t.value) e.items.push({
						start: [],
						key: null,
						sep: [this.sourceToken]
					});
					else if ($(t.sep, "map-value-ind")) this.stack.push({
						type: "block-map",
						offset: this.offset,
						indent: this.indent,
						items: [{
							start: i,
							key: null,
							sep: [this.sourceToken]
						}]
					});
					else if (Bl(t.key) && !$(t.sep, "newline")) {
						let e = Hl(t.start), n = t.key, r = t.sep;
						r.push(this.sourceToken), delete t.key, delete t.sep, this.stack.push({
							type: "block-map",
							offset: this.offset,
							indent: this.indent,
							items: [{
								start: e,
								key: n,
								sep: r
							}]
						});
					} else i.length > 0 ? t.sep = t.sep.concat(i, this.sourceToken) : t.sep.push(this.sourceToken);
					else t.sep ? t.value || r ? e.items.push({
						start: i,
						key: null,
						sep: [this.sourceToken]
					}) : $(t.sep, "map-value-ind") ? this.stack.push({
						type: "block-map",
						offset: this.offset,
						indent: this.indent,
						items: [{
							start: [],
							key: null,
							sep: [this.sourceToken]
						}]
					}) : t.sep.push(this.sourceToken) : Object.assign(t, {
						key: null,
						sep: [this.sourceToken]
					});
					this.onKeyLine = !0;
					return;
				case "alias":
				case "scalar":
				case "single-quoted-scalar":
				case "double-quoted-scalar": {
					let n = this.flowScalar(this.type);
					r || t.value ? (e.items.push({
						start: i,
						key: n,
						sep: []
					}), this.onKeyLine = !0) : t.sep ? this.stack.push(n) : (Object.assign(t, {
						key: n,
						sep: []
					}), this.onKeyLine = !0);
					return;
				}
				default: {
					let r = this.startBlockValue(e);
					if (r) {
						if (r.type === "block-seq") {
							if (!t.explicitKey && t.sep && !$(t.sep, "newline")) {
								yield* this.pop({
									type: "error",
									offset: this.offset,
									message: "Unexpected block-seq-ind on same line with key",
									source: this.source
								});
								return;
							}
						} else n && e.items.push({ start: i });
						this.stack.push(r);
						return;
					}
				}
			}
		}
		yield* this.pop(), yield* this.step();
	}
	*blockSequence(e) {
		let t = e.items[e.items.length - 1];
		switch (this.type) {
			case "newline":
				if (t.value) {
					let n = "end" in t.value ? t.value.end : void 0;
					(Array.isArray(n) ? n[n.length - 1] : void 0)?.type === "comment" ? n?.push(this.sourceToken) : e.items.push({ start: [this.sourceToken] });
				} else t.start.push(this.sourceToken);
				return;
			case "space":
			case "comment":
				if (t.value) e.items.push({ start: [this.sourceToken] });
				else {
					if (this.atIndentedComment(t.start, e.indent)) {
						let n = e.items[e.items.length - 2]?.value?.end;
						if (Array.isArray(n)) {
							Ul(n, t.start), n.push(this.sourceToken), e.items.pop();
							return;
						}
					}
					t.start.push(this.sourceToken);
				}
				return;
			case "anchor":
			case "tag":
				if (t.value || this.indent <= e.indent) break;
				t.start.push(this.sourceToken);
				return;
			case "seq-item-ind":
				if (this.indent !== e.indent) break;
				t.value || $(t.start, "seq-item-ind") ? e.items.push({ start: [this.sourceToken] }) : t.start.push(this.sourceToken);
				return;
		}
		if (this.indent > e.indent) {
			let t = this.startBlockValue(e);
			if (t) {
				this.stack.push(t);
				return;
			}
		}
		yield* this.pop(), yield* this.step();
	}
	*flowCollection(e) {
		let t = e.items[e.items.length - 1];
		if (this.type === "flow-error-end") {
			let e;
			do
				yield* this.pop(), e = this.peek(1);
			while (e?.type === "flow-collection");
		} else if (e.end.length === 0) {
			switch (this.type) {
				case "comma":
				case "explicit-key-ind":
					!t || t.sep ? e.items.push({ start: [this.sourceToken] }) : t.start.push(this.sourceToken);
					return;
				case "map-value-ind":
					!t || t.value ? e.items.push({
						start: [],
						key: null,
						sep: [this.sourceToken]
					}) : t.sep ? t.sep.push(this.sourceToken) : Object.assign(t, {
						key: null,
						sep: [this.sourceToken]
					});
					return;
				case "space":
				case "comment":
				case "newline":
				case "anchor":
				case "tag":
					!t || t.value ? e.items.push({ start: [this.sourceToken] }) : t.sep ? t.sep.push(this.sourceToken) : t.start.push(this.sourceToken);
					return;
				case "alias":
				case "scalar":
				case "single-quoted-scalar":
				case "double-quoted-scalar": {
					let n = this.flowScalar(this.type);
					!t || t.value ? e.items.push({
						start: [],
						key: n,
						sep: []
					}) : t.sep ? this.stack.push(n) : Object.assign(t, {
						key: n,
						sep: []
					});
					return;
				}
				case "flow-map-end":
				case "flow-seq-end":
					e.end.push(this.sourceToken);
					return;
			}
			let n = this.startBlockValue(e);
			/* istanbul ignore else should not happen */
			n ? this.stack.push(n) : (yield* this.pop(), yield* this.step());
		} else {
			let t = this.peek(2);
			if (t.type === "block-map" && (this.type === "map-value-ind" && t.indent === e.indent || this.type === "newline" && !t.items[t.items.length - 1].sep)) yield* this.pop(), yield* this.step();
			else if (this.type === "map-value-ind" && t.type !== "flow-collection") {
				let n = Hl(Vl(t));
				Wl(e);
				let r = e.end.splice(1, e.end.length);
				r.push(this.sourceToken);
				let i = {
					type: "block-map",
					offset: e.offset,
					indent: e.indent,
					items: [{
						start: n,
						key: e,
						sep: r
					}]
				};
				this.onKeyLine = !0, this.stack[this.stack.length - 1] = i;
			} else yield* this.lineEnd(e);
		}
	}
	flowScalar(e) {
		if (this.onNewLine) {
			let e = this.source.indexOf("\n") + 1;
			for (; e !== 0;) this.onNewLine(this.offset + e), e = this.source.indexOf("\n", e) + 1;
		}
		return {
			type: e,
			offset: this.offset,
			indent: this.indent,
			source: this.source
		};
	}
	startBlockValue(e) {
		switch (this.type) {
			case "alias":
			case "scalar":
			case "single-quoted-scalar":
			case "double-quoted-scalar": return this.flowScalar(this.type);
			case "block-scalar-header": return {
				type: "block-scalar",
				offset: this.offset,
				indent: this.indent,
				props: [this.sourceToken],
				source: ""
			};
			case "flow-map-start":
			case "flow-seq-start": return {
				type: "flow-collection",
				offset: this.offset,
				indent: this.indent,
				start: this.sourceToken,
				items: [],
				end: []
			};
			case "seq-item-ind": return {
				type: "block-seq",
				offset: this.offset,
				indent: this.indent,
				items: [{ start: [this.sourceToken] }]
			};
			case "explicit-key-ind": {
				this.onKeyLine = !0;
				let t = Hl(Vl(e));
				return t.push(this.sourceToken), {
					type: "block-map",
					offset: this.offset,
					indent: this.indent,
					items: [{
						start: t,
						explicitKey: !0
					}]
				};
			}
			case "map-value-ind": {
				this.onKeyLine = !0;
				let t = Hl(Vl(e));
				return {
					type: "block-map",
					offset: this.offset,
					indent: this.indent,
					items: [{
						start: t,
						key: null,
						sep: [this.sourceToken]
					}]
				};
			}
		}
		return null;
	}
	atIndentedComment(e, t) {
		return this.type !== "comment" || this.indent <= t ? !1 : e.every((e) => e.type === "newline" || e.type === "space");
	}
	*documentEnd(e) {
		this.type !== "doc-mode" && (e.end ? e.end.push(this.sourceToken) : e.end = [this.sourceToken], this.type === "newline" && (yield* this.pop()));
	}
	*lineEnd(e) {
		switch (this.type) {
			case "comma":
			case "doc-start":
			case "doc-end":
			case "flow-seq-end":
			case "flow-map-end":
			case "map-value-ind":
				yield* this.pop(), yield* this.step();
				break;
			case "newline": this.onKeyLine = !1;
			default: e.end ? e.end.push(this.sourceToken) : e.end = [this.sourceToken], this.type === "newline" && (yield* this.pop());
		}
	}
};
function Kl(e) {
	let t = e.prettyErrors !== !1;
	return {
		lineCounter: e.lineCounter || t && new Rl() || null,
		prettyErrors: t
	};
}
function ql(e, t = {}) {
	let { lineCounter: n, prettyErrors: r } = Kl(t), i = new Gl(n?.addNewLine), a = new Tl(t), o = null;
	for (let t of a.compose(i.parse(e), !0, e.length)) if (!o) o = t;
	else if (o.options.logLevel !== "silent") {
		o.errors.push(new Vc(t.range.slice(0, 2), "MULTIPLE_DOCS", "Source contains multiple documents; please use YAML.parseAllDocuments()"));
		break;
	}
	return r && n && (o.errors.forEach(Uc(e, n)), o.warnings.forEach(Uc(e, n))), o;
}
function Jl(e, t, n) {
	let r;
	typeof t == "function" ? r = t : n === void 0 && t && typeof t == "object" && (n = t);
	let i = ql(e, n);
	if (!i) return null;
	if (i.warnings.forEach((e) => Cs(i.options.logLevel, e)), i.errors.length > 0) {
		if (i.options.logLevel !== "silent") throw i.errors[0];
		i.errors = [];
	}
	return i.toJS(Object.assign({ reviver: r }, n));
}
var Yl = "---";
function Xl(e) {
	if (!e.startsWith(Yl + "\n") && !e.startsWith(Yl + "\r\n")) return {
		meta: void 0,
		body: e
	};
	let t = e.indexOf("\n"), n = e.slice(t + 1), r = n.search(/^---\s*$/m);
	if (r < 0) return {
		meta: void 0,
		body: e
	};
	let i = n.slice(0, r), a = n.slice(r), o = a.indexOf("\n"), s = o >= 0 ? a.slice(o + 1) : "", c;
	try {
		let e = Jl(i);
		c = e && typeof e == "object" && !Array.isArray(e) ? e : void 0;
	} catch {
		c = void 0;
	}
	return {
		meta: c,
		body: s
	};
}
//#endregion
//#region src/invariants.ts
function Zl(e) {
	return typeof e == "object" && !!e && !Array.isArray(e);
}
function Ql(e, t) {
	for (let n of e) if (Zl(n) && n.templateId === t && typeof n.path == "string" && typeof n.contentHash == "string" && typeof n.renderedAt == "string") return {
		templateId: t,
		path: n.path,
		contentHash: n.contentHash,
		renderedAt: n.renderedAt
	};
}
function $l(e) {
	return e === null ? "null" : e === void 0 ? "undefined" : Array.isArray(e) ? "array" : typeof e;
}
var eu = ({ definition: t, instance: n, arg: r }) => {
	let i = r;
	if (!i) return {
		ok: !1,
		code: "invariant_template_unknown",
		details: { reason: "argument required: artifact.frontmatter_valid:<templateId>" }
	};
	let a = t.artifactTemplates?.[i];
	if (!a) return {
		ok: !1,
		code: "invariant_template_unknown",
		details: { templateId: i }
	};
	if (!a.frontmatterSchema) return {
		ok: !1,
		code: "invariant_schema_missing",
		details: { templateId: i }
	};
	let o = t.itemSchemas[a.frontmatterSchema];
	if (!Zl(o)) return {
		ok: !1,
		code: "invariant_schema_missing",
		details: {
			templateId: i,
			schemaRef: a.frontmatterSchema,
			reason: "schema reference not found in itemSchemas"
		}
	};
	let s = o, c = Ql(n.artifacts, i);
	if (!c) return {
		ok: !1,
		code: "invariant_artifact_missing",
		details: { templateId: i }
	};
	let l;
	try {
		l = e(c.path, "utf8");
	} catch (e) {
		return {
			ok: !1,
			code: "invariant_artifact_missing",
			details: {
				templateId: i,
				path: c.path,
				cause: e.message
			}
		};
	}
	let { meta: u } = Xl(l);
	if (!u) return {
		ok: !1,
		code: "invariant_frontmatter_invalid",
		details: {
			templateId: i,
			path: c.path,
			reason: "no frontmatter"
		}
	};
	let d = [];
	for (let e of s.required ?? []) {
		let t = u[e];
		(t == null || typeof t == "string" && t.length === 0 || Array.isArray(t) && t.length === 0) && d.push(e);
	}
	if (d.length > 0) return {
		ok: !1,
		code: "invariant_frontmatter_invalid",
		details: {
			templateId: i,
			path: c.path,
			missing: d
		}
	};
	let f = [];
	for (let [e, t] of Object.entries(s.properties ?? {})) {
		if (!(e in u)) continue;
		let n = $l(u[e]);
		n !== t.type && f.push({
			key: e,
			expected: t.type,
			actual: n
		});
	}
	return f.length > 0 ? {
		ok: !1,
		code: "invariant_frontmatter_invalid",
		details: {
			templateId: i,
			path: c.path,
			typeMismatches: f
		}
	} : !0;
}, tu = /^ix:\/\/([^/]+)\/([^/]+)\/([^/]+)$/, nu = ({ definition: t, instance: n, arg: r }) => {
	let i = r;
	if (!i) return {
		ok: !1,
		code: "invariant_template_unknown",
		details: { reason: "argument required" }
	};
	if (!t.artifactTemplates?.[i]) return {
		ok: !1,
		code: "invariant_template_unknown",
		details: { templateId: i }
	};
	let a = Ql(n.artifacts, i);
	if (!a) return {
		ok: !1,
		code: "invariant_artifact_missing",
		details: { templateId: i }
	};
	let o;
	try {
		o = e(a.path, "utf8");
	} catch (e) {
		return {
			ok: !1,
			code: "invariant_artifact_missing",
			details: {
				templateId: i,
				path: a.path,
				cause: e.message
			}
		};
	}
	let { meta: s } = Xl(o);
	if (!s) return !0;
	let c = s.relationships;
	if (!Array.isArray(c) || c.length === 0) return !0;
	let l = [], u = [], d = /* @__PURE__ */ new Set();
	for (let e of Object.values(n.items)) for (let t of e) Zl(t) && typeof t.id == "string" && d.add(t.id);
	for (let e of c) {
		let t;
		if (typeof e == "string" ? t = e : Zl(e) && (t = e.target), typeof t != "string") {
			l.push(String(t));
			continue;
		}
		let r = tu.exec(t);
		if (!r) {
			l.push(t);
			continue;
		}
		let i = r[3];
		if (d.has(i)) continue;
		let a = ru(n);
		a && au(a, i) || u.push(t);
	}
	return l.length > 0 ? {
		ok: !1,
		code: "invariant_relationship_uri_invalid",
		details: {
			templateId: i,
			path: a.path,
			invalidUris: l
		}
	} : u.length > 0 ? {
		ok: !1,
		code: "invariant_relationship_unresolved",
		details: {
			templateId: i,
			path: a.path,
			unresolved: u
		}
	} : !0;
};
function ru(e) {
	for (let t of e.targets) if (Zl(t)) {
		let e = t;
		if (typeof e.repoDirectory == "string") return e.repoDirectory;
	}
}
var iu = /* @__PURE__ */ new Map();
function au(e, t) {
	let n = iu.get(e);
	return n || (n = ou(e), iu.set(e, n)), n.has(t);
}
function ou(e) {
	let n = /* @__PURE__ */ new Set(), r = /* @__PURE__ */ new Set(), i = [`${e}/spec`, `${e}/specs`];
	for (; i.length > 0;) {
		let e = i.pop();
		if (r.has(e)) continue;
		r.add(e);
		let a;
		try {
			a = t(e, { withFileTypes: !0 }).map((e) => ({
				name: e.name,
				isDir: e.isDirectory()
			}));
		} catch {
			continue;
		}
		for (let t of a) {
			let r = `${e}/${t.name}`;
			if (t.isDir) {
				i.push(r);
				continue;
			}
			if (!t.name.endsWith(".md")) continue;
			let a = /^([A-Z][A-Za-z]*-[0-9]+)[-.]/.exec(t.name);
			a && n.add(a[1]);
		}
	}
	return n;
}
function su() {
	iu.clear();
}
var cu = {
	"artifact.frontmatter_valid": eu,
	"artifact.relationships_resolve": nu
}, lu = [
	{
		id: "spec-matrix",
		path: "skills/matrix",
		workflows: ["matrix"],
		category: "review"
	},
	{
		id: "spec-review",
		path: "skills/review",
		workflows: ["review"],
		category: "review"
	},
	{
		id: "spec-object-review",
		path: "skills/object-review",
		workflows: ["object-review"],
		category: "review"
	},
	{
		id: "spec-app-review",
		path: "skills/app-review",
		workflows: ["app-review"],
		category: "review"
	},
	{
		id: "spec-security-analysis",
		path: "skills/security-analysis",
		workflows: ["security-analysis"],
		category: "analysis"
	},
	{
		id: "spec-to-plan",
		path: "skills/to-plan",
		workflows: ["to-plan"],
		category: "planning"
	}
];
function uu(e) {
	return lu.find((t) => t.id === e);
}
function du(e) {
	return lu.find((t) => t.workflows.includes(e))?.path;
}
//#endregion
export { eu as artifactFrontmatterValid, nu as artifactRelationshipsResolve, su as clearProjectRegistryCache, uu as skillById, cu as specInvariants, lu as specSkills, du as workflowSkillPath };
