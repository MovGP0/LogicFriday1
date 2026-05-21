/* 0044065a _rewind */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    _rewind
   
   Library: Visual Studio 2003 Release */

void __cdecl _rewind(FILE *_File)

{
  uint _FileHandle;
  undefined *puVar1;
  
  _FileHandle = _File->_file;
  __lock_file(_File);
  __flush(_File);
  _File->_flag = _File->_flag & 0xffffffcf;
  if (_FileHandle == 0xffffffff) {
    puVar1 = &DAT_00452260;
  }
  else {
    puVar1 = (undefined *)((&DAT_0046cc40)[(int)_FileHandle >> 5] + (_FileHandle & 0x1f) * 0x24);
  }
  puVar1[4] = puVar1[4] & 0xfd;
  if ((char)_File->_flag < '\0') {
    _File->_flag = _File->_flag & 0xfffffffc;
  }
  __lseek(_FileHandle,0,0);
  FUN_004406d9();
  return;
}
