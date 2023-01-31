import React, { Component } from 'react';
import { Button } from 'antd';
import './App.css';
import history from './history';
import { BrowserRouter as Router, Switch, Route, Link } from "react-router-dom";
import { useHistory } from "react-router-dom";
import TopMenu from './components/TopMenu';
import Track from './pages/track/Track';

const About = () => {
  return (
    <div>
      <h1>About</h1>
      <Button type="primary" onClick={() => {
        console.log("on click");
      }}>Button</Button>

    </div>
  )
}

export default () => {
  return (
    <Router>
      <div>
        <TopMenu />
        <Switch>
          <Route path="/about">
            <About></About>
          </Route>
          <Route path="/track">
            <Track></Track>
          </Route>
        </Switch>
      </div>
    </Router>
  );
};
