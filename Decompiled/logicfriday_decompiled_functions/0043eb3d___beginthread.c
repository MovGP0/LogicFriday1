/* 0043eb3d __beginthread */

/* Library Function - Single Match
    __beginthread
   
   Library: Visual Studio 2003 Release */

uintptr_t __cdecl __beginthread(_StartAddress *_StartAddress,uint _StackSize,void *_ArgList)

{
  int *piVar1;
  _ptiddata _Ptd;
  HANDLE hThread;
  DWORD DVar2;
  pthreadlocinfo unaff_ESI;
  
  DVar2 = 0;
  if (_StartAddress == (_StartAddress *)0x0) {
    piVar1 = FUN_00441a24();
    *piVar1 = 0x16;
  }
  else {
    _Ptd = _calloc(1,0x88);
    if (_Ptd != (_ptiddata)0x0) {
      __initptd(_Ptd,unaff_ESI);
      *(_StartAddress **)_Ptd->_con_ch_buf = _StartAddress;
      *(void **)(_Ptd->_con_ch_buf + 4) = _ArgList;
      hThread = CreateThread((LPSECURITY_ATTRIBUTES)0x0,_StackSize,
                             (LPTHREAD_START_ROUTINE)&UNK_0043ea9d,_Ptd,4,&_Ptd->_tid);
      _Ptd->_thandle = (uintptr_t)hThread;
      if ((hThread != (HANDLE)0x0) && (DVar2 = ResumeThread(hThread), DVar2 != 0xffffffff)) {
        return (uintptr_t)hThread;
      }
      DVar2 = GetLastError();
    }
    _free(_Ptd);
    if (DVar2 != 0) {
      __dosmaperr(DVar2);
    }
  }
  return 0xffffffff;
}
