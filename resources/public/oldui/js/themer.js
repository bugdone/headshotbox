function setActiveStyleSheet(title) {
  var i, a, found = false;
  $('link[rel*=style][title]').each(function() {
    this.disabled = true;
    if (this.getAttribute("title") == title) {
        this.disabled = false;
        found = true;
      }
  });
  if (!found)
    setActiveStyleSheet(getPreferredStyleSheet());
  else
    createCookie("style", title, 365);
}

function getPreferredStyleSheet() {
    return $('link[rel=stylesheet][title]').attr('title');
}

function createCookie(name,value,days) {
  if (days) {
    var date = new Date();
    date.setTime(date.getTime()+(days*24*60*60*1000));
    var expires = "; expires="+date.toGMTString();
  } else {
    expires = "";
  }
  document.cookie = name+"="+value+expires+"; path=/";
}

function readCookie(name) {
  var nameEQ = name + "=";
  var ca = document.cookie.split(';');
  for (var i=0;i < ca.length;i++) {
    var c = ca[i];
    while (c.charAt(0)==' ')
      c = c.substring(1,c.length);
    if (c.indexOf(nameEQ) == 0)
      return c.substring(nameEQ.length,c.length);
  }
  return null;
}

$(function() {
  setActiveStyleSheet(readCookie("style"));
});
