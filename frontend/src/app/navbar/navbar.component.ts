import { Component, OnInit } from '@angular/core';
import { ApiService } from '../api.service';

function cmpVersions(a: string, b: string): number {
  let regExStrip0 = /(\.0+)+$/;
  let segmentsA = a.replace(regExStrip0, '').split('.');
  let segmentsB = b.replace(regExStrip0, '').split('.');
  let l = Math.min(segmentsA.length, segmentsB.length);

  for (let i = 0; i < l; i++) {
    let diff = parseInt(segmentsA[i], 10) - parseInt(segmentsB[i], 10);
    if (diff) {
      return diff;
    }
  }
  return segmentsA.length - segmentsB.length;
}

@Component({
  selector: 'app-navbar',
  templateUrl: './navbar.component.html',
  styleUrls: ['./navbar.component.css']
})
export class NavbarComponent implements OnInit {
  showLogin = false;
  isAuthorized = false;
  version = '';
  newVersionAvailable = false;

  constructor(private api: ApiService) { }

  ngOnInit() {
    this.checkVersion();
    this.getAuthorizationState();
  }

  getAuthorizationState(): void {
    this.api.getAuthorization().then(data => {
      this.isAuthorized = data.authorized;
      this.showLogin = data.showLogin;
    });
  }

  checkVersion(): void {
    this.api.getVersion().then(data => {
      this.version = data.current;
      if (cmpVersions(data.current, data.latest) < 0) {
        this.newVersionAvailable = true;
      }
      setTimeout(() => this.checkVersion(), 1000 * 3600 * 24);
    });
  }
}
