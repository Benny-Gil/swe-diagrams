// Levels: a click or Enter on a node with a zoom mark (data-z) opens its
// detail level. The hash holds the path, so the back button goes up.
(function () {
  "use strict";
  var sd = window.sdViewer;
  var data = sd.data;
  if (!data.levels || data.levels.length < 2) return;
  var view = document.querySelector(".sd-view");
  var nav = document.querySelector(".sd-crumbs");
  var still = window.matchMedia("(prefers-reduced-motion: reduce)");
  var levels = {};
  data.levels.forEach(function (l) {
    levels[l.path] = l;
  });
  levels[""].el = sd.svg();
  levels[""].order = data.order;
  var cur = "";
  var busy = false;
  // The paths of the history entries of this page, so a move up can go back.
  var trail = [];

  function svgOf(lv) {
    if (!lv.el) {
      var t = document.createElement("template");
      t.innerHTML = lv.svg.trim();
      lv.el = t.content.firstElementChild;
    }
    return lv.el;
  }

  function parentOf(path) {
    var k = path.lastIndexOf("/");
    return k < 0 ? "" : path.slice(0, k);
  }

  function q(v) {
    return '"' + String(v).replace(/["\\]/g, "\\$&") + '"';
  }

  function zoomNode(id) {
    return sd.svg().querySelector("[data-z][data-id=" + q(id) + "]");
  }

  // Shows level `path` with no motion.
  function put(path) {
    var lv = levels[path];
    var el = svgOf(lv);
    var old = sd.svg();
    if (old !== el) view.replaceChild(el, old);
    data.order = lv.order;
    sd.mount(el);
    cur = path;
    el.querySelectorAll("[data-z]").forEach(function (n) {
      n.setAttribute("tabindex", "0");
      n.setAttribute("role", "button");
      var name = n.querySelector(".nm");
      n.setAttribute("aria-label", "Open the detail of " + (name ? name.textContent : n.dataset.id));
    });
    crumbs();
  }

  function title(path) {
    var t = levels[path].title;
    return t || (path ? path.split("/").pop() : data.type + " diagram");
  }

  function crumbs() {
    nav.textContent = "";
    var parts = cur ? cur.split("/") : [];
    for (var k = 0; k <= parts.length; k++) {
      var path = parts.slice(0, k).join("/");
      if (k) {
        var sep = document.createElement("span");
        sep.textContent = "\u203a";
        sep.setAttribute("aria-hidden", "true");
        nav.appendChild(sep);
      }
      var b = document.createElement(k < parts.length ? "button" : "span");
      b.textContent = title(path);
      if (k < parts.length) {
        b.type = "button";
        b.dataset.path = path;
      } else b.setAttribute("aria-current", "page");
      nav.appendChild(b);
    }
  }

  // A box around the node box, with the shape of the view.
  function around(r) {
    var v = sd.box();
    var a = v.w / v.h;
    var w = Math.max(r.width * 2.4, r.height * 2.4 * a);
    var h = w / a;
    return { x: r.x + r.width / 2 - w / 2, y: r.y + r.height / 2 - h / 2, w: w, h: h };
  }

  function ease(t) {
    return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
  }

  // Moves the view box from `a` to `b` in `ms`, then calls `done`.
  function glide(a, b, ms, done) {
    var t0 = performance.now();
    function frame(now) {
      var t = Math.min((now - t0) / ms, 1);
      var e = ease(t);
      sd.box({ x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, w: a.w + (b.w - a.w) * e, h: a.h + (b.h - a.h) * e });
      if (t < 1) requestAnimationFrame(frame);
      else done();
    }
    requestAnimationFrame(frame);
  }

  // The view zooms into the node, then the detail level grows from the
  // node box to the full view.
  function down(path, id) {
    var n = zoomNode(id);
    if (still.matches || !n) return finish(path);
    glide(sd.box(), around(n.getBBox()), 150, function () {
      var r = n.getBoundingClientRect();
      var v = view.getBoundingClientRect();
      put(path);
      var el = sd.svg();
      el.style.transformOrigin = "0 0";
      el.style.transform =
        "translate(" + (r.left - v.left) + "px," + (r.top - v.top) + "px) scale(" + r.width / v.width + "," + r.height / v.height + ")";
      el.style.opacity = "0.2";
      el.getBoundingClientRect();
      el.style.transition = "transform .2s ease-out, opacity .2s ease-out";
      el.style.transform = "";
      el.style.opacity = "";
      setTimeout(function () {
        el.style.transition = el.style.transformOrigin = "";
        finish(path);
      }, 210);
    });
  }

  // The parent level shows, zoomed into the node, and the view goes out.
  function up(path, id) {
    put(path);
    var n = zoomNode(id);
    if (!n) return finish(path);
    if (still.matches) {
      n.focus();
      return finish(path);
    }
    var home = sd.home();
    glide(around(n.getBBox()), home, 260, function () {
      n.focus({ preventScroll: true });
      finish(path);
    });
  }

  function finish(path) {
    if (cur !== path) put(path);
    busy = false;
    sync();
  }

  function wanted() {
    var p = decodeURIComponent(location.hash.replace(/^#/, ""));
    return levels[p] ? p : "";
  }

  // Goes to the level in the hash.
  function sync() {
    if (busy) return;
    var to = wanted();
    if (to === cur) return;
    busy = true;
    if (parentOf(to) === cur && to) down(to, to.split("/").pop());
    else if (cur.indexOf(to ? to + "/" : "") === 0) up(to, cur.slice(to ? to.length + 1 : 0).split("/")[0]);
    else finish(to);
  }

  function open(path) {
    if (trail.length > 1 && trail[trail.length - 2] === path) return history.back();
    location.hash = path ? "#" + path.split("/").map(encodeURIComponent).join("/") : "#";
  }

  view.addEventListener("click", function (e) {
    var n = e.target.closest("[data-z]");
    if (n && !e.target.closest("[data-sd-st]") && !sd.moved()) open(cur ? cur + "/" + n.dataset.id : n.dataset.id);
  });

  nav.addEventListener("click", function (e) {
    var b = e.target.closest("button");
    if (b) open(b.dataset.path);
  });

  document.addEventListener("keydown", function (e) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    var n = e.target.closest && e.target.closest("[data-z]");
    if (n && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      open(cur ? cur + "/" + n.dataset.id : n.dataset.id);
    } else if (e.key === "Escape" && cur) open(parentOf(cur));
  });

  window.addEventListener("hashchange", function () {
    var p = wanted();
    if (trail.length > 1 && trail[trail.length - 2] === p) trail.pop();
    else trail.push(p);
    sync();
  });
  nav.hidden = false;
  document.body.classList.add("sd-levels");
  if (!location.hash && data.start) open(data.start);
  put(wanted());
  trail.push(cur);
})();
