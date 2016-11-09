import { Component, OnInit } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { DemoFilters } from '../api.service';

@Component({
  selector: 'app-player',
  templateUrl: './player.component.html',
  styleUrls: ['./player.component.css']
})
export class PlayerComponent implements OnInit {
  steamid: string;
  filters = new DemoFilters();

  constructor(private route: ActivatedRoute) { }

  ngOnInit() {
    this.steamid = this.route.snapshot.params['steamid'];
  }

  filtersChanged(filters: DemoFilters) {
    console.log('new filters', filters);
  }
}
