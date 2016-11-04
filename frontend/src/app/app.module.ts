import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { HttpModule } from '@angular/http';
import { RouterModule } from '@angular/router';

import { DropdownModule, PaginationModule } from 'ng2-bootstrap/ng2-bootstrap';

import { ApiService } from './api.service';
import { TimestampPipe } from './timestamp.pipe';
import { AppComponent } from './app.component';
import { NavbarComponent } from './navbar/navbar.component';
import { PlayerListComponent } from './player-list/player-list.component';
import { DropdownComponent } from './dropdown/dropdown.component';

@NgModule({
  declarations: [
    AppComponent,
    TimestampPipe,
    PlayerListComponent,
    NavbarComponent,
    DropdownComponent
  ],
  imports: [
    BrowserModule,
    FormsModule,
    HttpModule,
    DropdownModule,
    PaginationModule,
    RouterModule.forRoot([
      { path: 'player_list', component: PlayerListComponent },
      { path: '', pathMatch: 'full', redirectTo: 'player_list' }
    ])
  ],
  providers: [ApiService],
  bootstrap: [AppComponent]
})
export class AppModule { }
