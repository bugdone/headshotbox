import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { HttpModule } from '@angular/http';
import { RouterModule } from '@angular/router';
import { DecimalPipe } from '@angular/common';

import { DropdownModule, PaginationModule, DatepickerModule } from 'ng2-bootstrap/ng2-bootstrap';

import { ApiService } from './api.service';
import { TimestampPipe } from './timestamp.pipe';
import { AppComponent } from './app.component';
import { NavbarComponent } from './navbar/navbar.component';
import { PlayerListComponent } from './player-list/player-list.component';
import { PlayerComponent } from './player/player.component';
import { NumericPipe } from './numeric.pipe';
import { DemoFiltersComponent } from './demo-filters/demo-filters.component';
import { DropdownComponent } from './dropdown/dropdown.component';
import { PlayerStatsComponent } from './player-stats/player-stats.component';
import { DatepickerComponent } from './datepicker/datepicker.component';
import { TeammatesPickerComponent } from './teammates-picker/teammates-picker.component';

@NgModule({
  declarations: [
    AppComponent,
    TimestampPipe,
    PlayerListComponent,
    NavbarComponent,
    PlayerComponent,
    NumericPipe,
    DemoFiltersComponent,
    DropdownComponent,
    PlayerStatsComponent,
    DatepickerComponent,
    TeammatesPickerComponent
  ],
  imports: [
    BrowserModule,
    FormsModule,
    HttpModule,
    DropdownModule,
    PaginationModule,
    DatepickerModule,
    RouterModule.forRoot([
      { path: 'player/:steamid', component: PlayerComponent },
      { path: 'player_list', component: PlayerListComponent },
      { path: '', pathMatch: 'full', redirectTo: 'player_list' }
    ])
  ],
  providers: [ApiService, DecimalPipe],
  bootstrap: [AppComponent]
})
export class AppModule { }
