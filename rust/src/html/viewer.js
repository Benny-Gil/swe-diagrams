// The swe-diagrams viewer: pan, zoom, hover, a status filter, and steps.
// The SVG marks each part with data-k (node, group, edge, msg).
(function () {
  "use strict";
  var view = document.querySelector(".sd-view");
  var data = JSON.parse(document.getElementById("sd-data").textContent);
  var WORDS = ["", "done", "wip", "plan", "add", "mod", "del"];
  var svg, parts, home, vb;
  var keys = [];

  // Shows the SVG `el`. A diagram with levels swaps the SVG.
  function mount(el) {
    svg = el;
    parts = svg.querySelectorAll("[data-k]");
    var base = svg.viewBox.baseVal;
    home = { x: base.x, y: base.y, w: base.width, h: base.height };
    vb = Object.assign({}, home);
    svg.removeAttribute("width");
    svg.removeAttribute("height");
    filter = "";
    step = -1;
    legend();
    show();
  }

  function show() {
    svg.setAttribute("viewBox", vb.x + " " + vb.y + " " + vb.w + " " + vb.h);
  }

  // A screen point in the units of the SVG.
  function point(cx, cy) {
    var m = svg.getScreenCTM();
    var p = new DOMPoint(cx, cy).matrixTransform(m.inverse());
    return { x: p.x, y: p.y, s: m.a };
  }

  // The point under (cx, cy) stays in place, so a zoom follows the pointer.
  function zoom(k, cx, cy) {
    var r = view.getBoundingClientRect();
    if (cx === undefined) {
      cx = r.left + r.width / 2;
      cy = r.top + r.height / 2;
    }
    var w = Math.min(Math.max(vb.w / k, home.w / 20), home.w * 4);
    k = vb.w / w;
    var p = point(cx, cy);
    vb.x = p.x - (p.x - vb.x) / k;
    vb.y = p.y - (p.y - vb.y) / k;
    vb.w = w;
    vb.h = vb.h / k;
    show();
  }

  function fit() {
    vb = Object.assign({}, home);
    show();
  }

  function pan(dx, dy) {
    var s = svg.getScreenCTM().a;
    vb.x -= dx / s;
    vb.y -= dy / s;
    show();
  }

  // Highlight: the parts in `on` stay bright and the rest dims. No list
  // clears the highlight.
  function light(on) {
    svg.classList.toggle("sd-dim", !!on);
    parts.forEach(function (el) {
      el.classList.remove("sd-on");
    });
    (on || []).forEach(function (el) {
      el.classList.add("sd-on");
    });
  }

  function all(sel) {
    return Array.prototype.slice.call(svg.querySelectorAll(sel));
  }

  function q(v) {
    return '"' + String(v).replace(/["\\]/g, "\\$&") + '"';
  }

  function node(id) {
    return all("[data-k=node][data-id=" + q(id) + "],[data-k=group][data-id=" + q(id) + "]");
  }

  // A node, its edges, and the nodes at the other end of them.
  function around(id) {
    var on = node(id);
    all("[data-from=" + q(id) + "],[data-to=" + q(id) + "]").forEach(function (e) {
      on.push(e);
      on = on.concat(node(e.dataset.from === id ? e.dataset.to : e.dataset.from));
    });
    return on;
  }

  var filter = "";
  var step = -1;

  // The lasting highlight: the status filter, else the current step.
  function rest() {
    if (filter) return light(all("[data-s=" + q(filter) + "]"));
    if (step >= 0) return light(stepParts(step));
    light(null);
  }

  function stepParts(k) {
    var s = data.order[k];
    if (s.k === "msg") {
      var m = all("[data-k=msg][data-i=" + q(s.i) + "]");
      var f = m[0] && m[0].dataset;
      return f ? m.concat(node(f.from), node(f.to)) : m;
    }
    return node(s.id);
  }

  // Moves the view when the current step is out of sight.
  function reveal(els) {
    var b = null;
    els.forEach(function (el) {
      var r = el.getBBox();
      if (!b) b = { x0: r.x, y0: r.y, x1: r.x + r.width, y1: r.y + r.height };
      else {
        b.x0 = Math.min(b.x0, r.x);
        b.y0 = Math.min(b.y0, r.y);
        b.x1 = Math.max(b.x1, r.x + r.width);
        b.y1 = Math.max(b.y1, r.y + r.height);
      }
    });
    if (!b) return;
    if (b.x0 < vb.x || b.y0 < vb.y || b.x1 > vb.x + vb.w || b.y1 > vb.y + vb.h) {
      vb.x = (b.x0 + b.x1) / 2 - vb.w / 2;
      vb.y = (b.y0 + b.y1) / 2 - vb.h / 2;
      show();
    }
  }

  function go(d) {
    var n = data.order.length;
    if (!n) return;
    step = Math.min(Math.max(step + d, 0), n - 1);
    filter = "";
    mark();
    var els = stepParts(step);
    light(els);
    reveal(els);
  }

  // Legend entries of the statuses become buttons.
  function legend() {
    keys = [];
    all("[data-sd-b]").forEach(function (u) {
      var n = +u.getAttribute("data-sd-b");
      if (!n || u.closest("[data-k]")) return;
      var word = WORDS[n];
      var label = u.nextElementSibling;
      [u, label].forEach(function (el) {
        if (!el) return;
        el.setAttribute("data-sd-st", word);
        keys.push(el);
      });
      u.setAttribute("tabindex", "0");
      u.setAttribute("role", "button");
      u.setAttribute("aria-label", "Show only " + (label ? label.textContent : word));
    });
  }

  function mark() {
    svg.classList.toggle("sd-filter", !!filter);
    keys.forEach(function (el) {
      el.classList.toggle("sd-sel", el.getAttribute("data-sd-st") === filter);
      if (el.getAttribute("role")) el.setAttribute("aria-pressed", String(el.getAttribute("data-sd-st") === filter));
    });
  }

  function pick(word) {
    filter = filter === word ? "" : word;
    step = -1;
    mark();
    rest();
  }

  // Pointer input: drag pans, two pointers pinch.
  var ptrs = new Map();
  var pinch = 0;
  var moved = false;

  view.addEventListener("pointerdown", function (e) {
    if (e.button !== 0 && e.pointerType === "mouse") return;
    ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
    moved = false;
    pinch = 0;
  });

  view.addEventListener("pointermove", function (e) {
    var p = ptrs.get(e.pointerId);
    if (!p) return;
    if (ptrs.size === 2) {
      var other = Array.from(ptrs.entries()).filter(function (kv) {
        return kv[0] !== e.pointerId;
      })[0][1];
      var d = Math.hypot(e.clientX - other.x, e.clientY - other.y);
      if (pinch) zoom(d / pinch, (e.clientX + other.x) / 2, (e.clientY + other.y) / 2);
      pinch = d;
    } else if (moved || Math.abs(e.clientX - p.x) + Math.abs(e.clientY - p.y) > 3) {
      // The capture starts only after a move, so a click still reaches its target.
      if (!moved) view.setPointerCapture(e.pointerId);
      moved = true;
      view.classList.add("sd-drag");
      pan(e.clientX - p.x, e.clientY - p.y);
    } else {
      return;
    }
    p.x = e.clientX;
    p.y = e.clientY;
  });

  function up(e) {
    ptrs.delete(e.pointerId);
    pinch = 0;
    if (!ptrs.size) view.classList.remove("sd-drag");
  }
  view.addEventListener("pointerup", up);
  view.addEventListener("pointercancel", up);

  view.addEventListener(
    "wheel",
    function (e) {
      e.preventDefault();
      var dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
      zoom(Math.exp(-dy * (e.ctrlKey ? 0.01 : 0.0015)), e.clientX, e.clientY);
    },
    { passive: false }
  );

  view.addEventListener("pointerover", function (e) {
    if (ptrs.size) return;
    var n = e.target.closest("[data-k=node]");
    if (n) light(around(n.dataset.id));
  });

  view.addEventListener("pointerout", function (e) {
    var n = e.target.closest("[data-k=node]");
    if (n && !(e.relatedTarget && n.contains(e.relatedTarget))) rest();
  });

  view.addEventListener("click", function (e) {
    var k = e.target.closest("[data-sd-st]");
    if (k && !moved) pick(k.getAttribute("data-sd-st"));
  });

  document.querySelector(".sd-tools").addEventListener("click", function (e) {
    var b = e.target.closest("button");
    if (!b) return;
    var act = b.getAttribute("data-act");
    if (act === "in") zoom(1.25);
    else if (act === "out") zoom(0.8);
    else fit();
  });

  document.addEventListener("keydown", function (e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    var k = e.target.closest && e.target.closest("[data-sd-st]");
    if (k && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      return pick(k.getAttribute("data-sd-st"));
    }
    if (e.target.tagName === "BUTTON" && (e.key === "Enter" || e.key === " ")) return;
    var done = true;
    if (e.key === "+" || e.key === "=") zoom(1.25);
    else if (e.key === "-" || e.key === "_") zoom(0.8);
    else if (e.key === "0") fit();
    else if (e.key === "ArrowRight" || e.key === "ArrowDown") go(1);
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") go(step < 0 ? 1 : -1);
    else if (e.key === "Escape") {
      filter = "";
      step = -1;
      mark();
      rest();
    } else done = false;
    if (done) e.preventDefault();
  });

  mount(view.querySelector("svg"));

  // The levels script moves the view and swaps the SVG through this object.
  window.sdViewer = {
    data: data,
    mount: mount,
    svg: function () {
      return svg;
    },
    box: function (b) {
      if (b) {
        vb = Object.assign({}, b);
        show();
      }
      return Object.assign({}, vb);
    },
    home: function () {
      return Object.assign({}, home);
    },
    moved: function () {
      return moved;
    },
  };
})();
