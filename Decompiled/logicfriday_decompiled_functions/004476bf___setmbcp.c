/* 004476bf __setmbcp */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __setmbcp
   
   Library: Visual Studio 2003 Release */

int __cdecl __setmbcp(int _CodePage)

{
  int *_Memory;
  int iVar1;
  int local_20;
  
  local_20 = -1;
  __lock(0xd);
  DAT_0046c9b0 = 0;
  if (_CodePage == -2) {
    DAT_0046c9b0 = 1;
    _CodePage = GetOEMCP();
  }
  else if (_CodePage == -3) {
    DAT_0046c9b0 = 1;
    _CodePage = GetACP();
  }
  else if (_CodePage == -4) {
    DAT_0046c9b0 = 1;
    _CodePage = DAT_0046c980;
  }
  if (_CodePage == DAT_0046cb04) {
    local_20 = 0;
  }
  else {
    if ((DAT_0046c9f8 == (int *)0x0) || (_Memory = DAT_0046c9f8, *DAT_0046c9f8 != 0)) {
      _Memory = _malloc(0x220);
    }
    if ((_Memory != (int *)0x0) && (local_20 = FUN_00447529(_CodePage), local_20 == 0)) {
      *_Memory = 0;
      _Memory[1] = DAT_0046cb04;
      _Memory[2] = DAT_0046c9fc;
      _Memory[3] = DAT_0046c9f4;
      for (iVar1 = 0; iVar1 < 5; iVar1 = iVar1 + 1) {
        *(undefined2 *)((int)_Memory + iVar1 * 2 + 0x10) = (&DAT_0046cb10)[iVar1];
      }
      for (iVar1 = 0; iVar1 < 0x101; iVar1 = iVar1 + 1) {
        *(undefined1 *)(iVar1 + 0x1c + (int)_Memory) = (&DAT_0046ca00)[iVar1];
      }
      for (iVar1 = 0; DAT_0046c9f8 = _Memory, iVar1 < 0x100; iVar1 = iVar1 + 1) {
        *(undefined1 *)(iVar1 + 0x11d + (int)_Memory) = (&DAT_0046cb20)[iVar1];
      }
    }
    if ((local_20 == -1) && (_Memory != DAT_0046c9f8)) {
      _free(_Memory);
    }
  }
  FUN_00447806();
  return local_20;
}
