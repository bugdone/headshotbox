import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { HttpModule } from '@angular/http';

import { ApiService } from './api.service';
import { AppComponent } from './app.component';
import { PlayerListComponent } from './player-list/player-list.component';
import { DropdownModule } from 'ng2-bootstrap/ng2-bootstrap';
import { TimestampPipe } from './timestamp.pipe';

@NgModule({
  declarations: [
    AppComponent,
    PlayerListComponent,
    TimestampPipe
  ],
  imports: [
    BrowserModule,
    FormsModule,
    HttpModule,
    DropdownModule
  ],
  providers: [ApiService],
  bootstrap: [AppComponent]
})
export class AppModule { }
