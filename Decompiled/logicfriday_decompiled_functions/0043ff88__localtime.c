/* 0043ff88 _localtime */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* Library Function - Single Match
    _localtime
   
   Library: Visual Studio 2003 Release */

tm * __cdecl _localtime(time_t *_Time)

{
  time_t *_Time_00;
  bool bVar1;
  tm *ptVar2;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  int iVar3;
  
  _Time_00 = _Time;
  if ((int)*_Time < 0) {
    ptVar2 = (tm *)0x0;
  }
  else {
    FUN_00444cbd();
    iVar3 = (int)*_Time_00;
    if ((iVar3 < 0x3f481) || (0x7ffc0b7e < iVar3)) {
      ptVar2 = _gmtime(_Time_00);
      if ((DAT_004521b4 == 0) || (bVar1 = FUN_00444ceb(), CONCAT31(extraout_var_00,bVar1) == 0)) {
        _Time = (time_t *)(ptVar2->tm_sec - _DAT_004521b0);
      }
      else {
        _Time = (time_t *)((ptVar2->tm_sec - DAT_004521b8) - _DAT_004521b0);
        ptVar2->tm_isdst = 1;
      }
      iVar3 = (int)_Time % 0x3c;
      ptVar2->tm_sec = iVar3;
      if (iVar3 < 0) {
        ptVar2->tm_sec = iVar3 + 0x3c;
        _Time = (time_t *)((int)_Time + -0x3c);
      }
      _Time = (time_t *)((int)_Time / 0x3c + ptVar2->tm_min);
      iVar3 = (int)_Time % 0x3c;
      ptVar2->tm_min = iVar3;
      if (iVar3 < 0) {
        ptVar2->tm_min = iVar3 + 0x3c;
        _Time = (time_t *)((int)_Time + -0x3c);
      }
      _Time = (time_t *)((int)_Time / 0x3c + ptVar2->tm_hour);
      iVar3 = (int)_Time % 0x18;
      ptVar2->tm_hour = iVar3;
      if (iVar3 < 0) {
        ptVar2->tm_hour = iVar3 + 0x18;
        _Time = _Time + -3;
      }
      iVar3 = (int)_Time / 0x18;
      if (iVar3 < 1) {
        if (-1 < iVar3) {
          return ptVar2;
        }
        ptVar2->tm_wday = (ptVar2->tm_wday + 7 + iVar3) % 7;
        ptVar2->tm_mday = ptVar2->tm_mday + iVar3;
        if (ptVar2->tm_mday < 1) {
          ptVar2->tm_year = ptVar2->tm_year + -1;
          ptVar2->tm_mday = ptVar2->tm_mday + 0x1f;
          ptVar2->tm_yday = 0x16c;
          ptVar2->tm_mon = 0xb;
          return ptVar2;
        }
      }
      else {
        ptVar2->tm_wday = (ptVar2->tm_wday + iVar3) % 7;
        ptVar2->tm_mday = ptVar2->tm_mday + iVar3;
      }
      ptVar2->tm_yday = ptVar2->tm_yday + iVar3;
    }
    else {
      _Time = (time_t *)(iVar3 - _DAT_004521b0);
      ptVar2 = _gmtime((time_t *)&_Time);
      if ((DAT_004521b4 != 0) && (bVar1 = FUN_00444ceb(), CONCAT31(extraout_var,bVar1) != 0)) {
        _Time = (time_t *)((int)_Time - DAT_004521b8);
        ptVar2 = _gmtime((time_t *)&_Time);
        ptVar2->tm_isdst = 1;
      }
    }
  }
  return ptVar2;
}
