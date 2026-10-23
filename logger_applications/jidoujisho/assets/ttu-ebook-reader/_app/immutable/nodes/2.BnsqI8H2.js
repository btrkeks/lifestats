import{n as d,s as l,e as p,m as u,d as o,f,c as h,a as _,i as m,o as b,j as r}from"../chunks/scheduler.CsvoxDVR.js";import{S as g,i as v}from"../chunks/index.WmXHMrZj.js";import{g as $}from"../chunks/entry.BDKO3PC4.js";import{d as y,m as P,p as x,t as E}from"../chunks/store.Cb9Wjk6F.js";import{f as I}from"../chunks/format-page-title.CL9ZAxoP.js";/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */function N(s,a){const e=a.subscribe();return{destroy:()=>e.unsubscribe()}}function S(s){let a,e,n,c;return document.title=I("Home"),{c(){a=l(),e=p("div")},l(t){u("svelte-1o30anf",document.head).forEach(o),a=f(t),e=h(t,"DIV",{}),_(e).forEach(o)},m(t,i){m(t,a,i),m(t,e,i),n||(c=b(N.call(null,e,s[0])),n=!0)},p:r,i:r,o:r,d(t){t&&(o(a),o(e)),n=!1,c()}}}function j(s){return[y.lastItem$.pipe(P(e=>e?`${x}/b?id=${e.dataId}`:"manage"),E($))]}class V extends g{constructor(a){super(),v(this,a,j,S,d,{})}}export{V as component};
